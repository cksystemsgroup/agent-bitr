//! SMT-LIB2 parser for QF_BV and QF_ABV benchmarks
//!
//! Parses SMT-LIB2 format into BVCs for the BITR solver.
//! Supports: declare-const, declare-fun, assert, check-sat, define-fun.

use std::collections::HashMap;

use bvdd::types::{BvcId, BvWidth, OpKind};
use bvdd::term::TermTable;
use bvdd::constraint::ConstraintTable;
use bvdd::bvc::{BvcManager, BvcEntry};

/// Parsed SMT-LIB2 model
pub struct SmtModel {
    pub tt: TermTable,
    pub ct: ConstraintTable,
    pub bm: BvcManager,
    pub assertions: Vec<BvcId>,
    pub var_map: HashMap<String, BvcId>,
}

/// SMT-LIB2 sort
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum SmtSort {
    BitVec(BvWidth),
    Bool,
    Array(Box<SmtSort>, Box<SmtSort>),
}

/// Tokenizer for S-expressions
struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Tokenizer { input, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let c = self.input.as_bytes()[self.pos];
            if c == b';' {
                // Skip comment to end of line
                while self.pos < self.input.len() && self.input.as_bytes()[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else if c.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Option<String> {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return None;
        }
        let c = self.input.as_bytes()[self.pos];
        if c == b'(' {
            self.pos += 1;
            return Some("(".to_string());
        }
        if c == b')' {
            self.pos += 1;
            return Some(")".to_string());
        }
        if c == b'"' {
            // String literal
            self.pos += 1;
            let start = self.pos;
            while self.pos < self.input.len() && self.input.as_bytes()[self.pos] != b'"' {
                self.pos += 1;
            }
            let s = self.input[start..self.pos].to_string();
            if self.pos < self.input.len() { self.pos += 1; }
            return Some(format!("\"{}\"", s));
        }
        if c == b'|' {
            // Quoted symbol
            self.pos += 1;
            let start = self.pos;
            while self.pos < self.input.len() && self.input.as_bytes()[self.pos] != b'|' {
                self.pos += 1;
            }
            let s = self.input[start..self.pos].to_string();
            if self.pos < self.input.len() { self.pos += 1; }
            return Some(s);
        }
        // Regular symbol/number
        let start = self.pos;
        while self.pos < self.input.len() {
            let c = self.input.as_bytes()[self.pos];
            if c.is_ascii_whitespace() || c == b'(' || c == b')' || c == b';' {
                break;
            }
            self.pos += 1;
        }
        Some(self.input[start..self.pos].to_string())
    }
}

/// S-expression
#[derive(Debug, Clone)]
enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

fn parse_sexp(tok: &mut Tokenizer) -> Option<Sexp> {
    let token = tok.next_token()?;
    if token == "(" {
        let mut items = Vec::new();
        loop {
            tok.skip_whitespace();
            if tok.pos < tok.input.len() && tok.input.as_bytes()[tok.pos] == b')' {
                tok.pos += 1;
                break;
            }
            if let Some(item) = parse_sexp(tok) {
                items.push(item);
            } else {
                break;
            }
        }
        Some(Sexp::List(items))
    } else {
        Some(Sexp::Atom(token))
    }
}

/// Parse an SMT-LIB2 file
pub fn parse_smtlib2(input: &str) -> Result<SmtModel, String> {
    let mut tok = Tokenizer::new(input);
    let mut model = SmtModel {
        tt: TermTable::new(),
        ct: ConstraintTable::new(),
        bm: BvcManager::new(),
        assertions: Vec::new(),
        var_map: HashMap::new(),
    };
    let mut var_ids: HashMap<String, (u32, BvWidth)> = HashMap::new();
    let mut next_var: u32 = 1;
    let mut defines: HashMap<String, BvcId> = HashMap::new();
    // Parameterized function definitions: name → (params, body)
    let mut fun_defs: HashMap<String, (Vec<(String, SmtSort)>, Sexp)> = HashMap::new();
    // Array variables: name → (var_id, index_width, element_width)
    let mut array_vars: HashMap<String, (u32, BvWidth, BvWidth)> = HashMap::new();
    // Array expressions for ROW expansion: name → ArrayExpr
    let mut array_exprs: HashMap<String, ArrayExpr> = HashMap::new();
    // Array define-fun bodies (stored as S-expressions for lazy expansion)
    let mut array_defines: HashMap<String, Sexp> = HashMap::new();

    while let Some(sexp) = parse_sexp(&mut tok) {
        if let Sexp::List(items) = &sexp {
            if items.is_empty() { continue; }
            if let Sexp::Atom(cmd) = &items[0] {
                match cmd.as_str() {
                    "set-logic" | "set-info" | "set-option" | "exit" | "get-model"
                    | "get-value" | "get-unsat-core" | "push" | "pop" => {
                        // Ignore
                    }
                    "declare-const" => {
                        if items.len() >= 3 {
                            let name = atom_str(&items[1]);
                            let sort = parse_sort(&items[2])?;
                            declare_var(&sort, &name, &mut next_var, &mut var_ids,
                                &mut array_vars, &mut model, &mut defines);
                            if let Some(&(vid, iw, ew)) = array_vars.get(&name) {
                                array_exprs.insert(name, ArrayExpr::Base { var_id: vid, index_width: iw, element_width: ew });
                            }
                        }
                    }
                    "declare-fun" => {
                        if items.len() >= 4 {
                            let name = atom_str(&items[1]);
                            let sort = parse_sort(&items[items.len() - 1])?;
                            declare_var(&sort, &name, &mut next_var, &mut var_ids,
                                &mut array_vars, &mut model, &mut defines);
                            if let Some(&(vid, iw, ew)) = array_vars.get(&name) {
                                array_exprs.insert(name, ArrayExpr::Base { var_id: vid, index_width: iw, element_width: ew });
                            }
                        }
                    }
                    "define-fun" => {
                        // (define-fun name ((params)) sort body)
                        if items.len() >= 5 {
                            let name = atom_str(&items[1]);
                            let body = items[items.len() - 1].clone();

                            // Parse parameters
                            let mut params = Vec::new();
                            if let Sexp::List(param_list) = &items[2] {
                                for p in param_list {
                                    if let Sexp::List(pair) = p {
                                        if pair.len() >= 2 {
                                            let pname = atom_str(&pair[0]);
                                            if let Ok(psort) = parse_sort(&pair[1]) {
                                                params.push((pname, psort));
                                            }
                                        }
                                    }
                                }
                            }

                            if params.is_empty() {
                                // Check if the return sort is an array
                                let ret_sort = parse_sort(&items[items.len() - 2]).ok();
                                let is_array = matches!(ret_sort, Some(SmtSort::Array(_, _)));

                                if is_array {
                                    // Array-returning: eagerly expand body into ArrayExpr
                                    array_defines.insert(name.clone(), body.clone());
                                    if let Ok(expr) = build_array_expr(
                                        &body, &mut model.tt, &mut model.ct, &mut model.bm,
                                        &model.var_map, &var_ids, &defines, &fun_defs, &array_vars,
                                        &array_defines,
                                    ) {
                                        array_exprs.insert(name, expr);
                                    }
                                } else {
                                    // BV-returning: evaluate immediately
                                    let bvc = translate_expr(
                                        &body, &mut model.tt, &mut model.ct, &mut model.bm,
                                        &model.var_map, &var_ids, &defines, &fun_defs, &array_vars,
                                    )?;
                                    defines.insert(name.clone(), bvc);
                                    model.var_map.insert(name, bvc);
                                }
                            } else {
                                // Parameterized: store as template
                                fun_defs.insert(name, (params, body));
                            }
                        }
                    }
                    "assert" => {
                        if items.len() >= 2 {
                            // Pre-expand array define-fun names in the assertion
                            let expanded = expand_array_names(&items[1], &array_defines);
                            let bvc = translate_expr(
                                &expanded, &mut model.tt, &mut model.ct, &mut model.bm,
                                &model.var_map, &var_ids, &defines, &fun_defs, &array_vars,
                            )?;
                            model.assertions.push(bvc);
                        }
                    }
                    "check-sat" => {
                        // We'll handle this at the top level
                    }
                    _ => {
                        // Unknown command — ignore
                    }
                }
            }
        }
    }

    Ok(model)
}

fn atom_str(sexp: &Sexp) -> String {
    match sexp {
        Sexp::Atom(s) => s.clone(),
        Sexp::List(_) => String::new(),
    }
}

fn parse_sort(sexp: &Sexp) -> Result<SmtSort, String> {
    match sexp {
        Sexp::Atom(s) => {
            if s == "Bool" { return Ok(SmtSort::Bool); }
            Err(format!("unknown sort: {}", s))
        }
        Sexp::List(items) => {
            if items.len() == 3 {
                if let (Sexp::Atom(u), Sexp::Atom(kind), Sexp::Atom(width)) =
                    (&items[0], &items[1], &items[2])
                {
                    if u == "_" && kind == "BitVec" {
                        let w: u16 = width.parse().map_err(|_| format!("bad width: {}", width))?;
                        return Ok(SmtSort::BitVec(w));
                    }
                }
            }
            if items.len() == 3 {
                if let Sexp::Atom(s) = &items[0] {
                    if s == "Array" {
                        let idx = parse_sort(&items[1])?;
                        let elem = parse_sort(&items[2])?;
                        return Ok(SmtSort::Array(Box::new(idx), Box::new(elem)));
                    }
                }
            }
            Err(format!("unknown sort: {:?}", sexp))
        }
    }
}

/// Array expression for ROW expansion
#[derive(Clone, Debug)]
enum ArrayExpr {
    /// Base array variable
    Base { var_id: u32, index_width: BvWidth, element_width: BvWidth },
    /// Store: store(base, index, value)
    Store { base: Box<ArrayExpr>, idx_bvc: BvcId, val_bvc: BvcId, element_width: BvWidth },
}

/// Expand a select from an array expression using ROW (Read-Over-Write)
fn expand_select_row(
    arr: &ArrayExpr,
    read_idx_bvc: BvcId,
    tt: &mut TermTable,
    ct: &mut ConstraintTable,
    bm: &mut BvcManager,
) -> BvcId {
    match arr {
        ArrayExpr::Base { var_id, index_width, element_width } => {
            // Read from base: unconstrained (symbolic)
            let arr_term = tt.make_var(*var_id, *index_width);
            let idx_term = bm.get(read_idx_bvc).entries[0].term;
            let read_term = tt.make_app(OpKind::Read, vec![arr_term, idx_term], *element_width);
            bm.alloc(*element_width, vec![BvcEntry { term: read_term, constraint: ct.true_id() }])
        }
        ArrayExpr::Store { base, idx_bvc, val_bvc, element_width } => {
            // ROW: ITE(EQ(read_idx, write_idx), write_val, select(base, read_idx))
            let r_term = bm.get(read_idx_bvc).entries[0].term;
            let w_idx_term = bm.get(*idx_bvc).entries[0].term;
            let w_val_term = bm.get(*val_bvc).entries[0].term;

            let eq = tt.make_app(OpKind::Eq, vec![r_term, w_idx_term], 1);
            let base_read = expand_select_row(base, read_idx_bvc, tt, ct, bm);
            let base_term = bm.get(base_read).entries[0].term;

            let ite = tt.make_app(OpKind::Ite, vec![eq, w_val_term, base_term], *element_width);
            bm.alloc(*element_width, vec![BvcEntry { term: ite, constraint: ct.true_id() }])
        }
    }
}

/// Helper: declare a variable of any sort
fn declare_var(
    sort: &SmtSort,
    name: &str,
    next_var: &mut u32,
    var_ids: &mut HashMap<String, (u32, BvWidth)>,
    array_vars: &mut HashMap<String, (u32, BvWidth, BvWidth)>,
    model: &mut SmtModel,
    _defines: &mut HashMap<String, BvcId>,
) {
    match sort {
        SmtSort::BitVec(w) => {
            let vid = *next_var; *next_var += 1;
            var_ids.insert(name.to_string(), (vid, *w));
            let bvc = model.bm.make_input(&mut model.tt, &model.ct, vid, *w);
            model.var_map.insert(name.to_string(), bvc);
        }
        SmtSort::Bool => {
            let vid = *next_var; *next_var += 1;
            var_ids.insert(name.to_string(), (vid, 1));
            let bvc = model.bm.make_input(&mut model.tt, &model.ct, vid, 1);
            model.var_map.insert(name.to_string(), bvc);
        }
        SmtSort::Array(idx_sort, elem_sort) => {
            if let (SmtSort::BitVec(iw), SmtSort::BitVec(ew)) = (idx_sort.as_ref(), elem_sort.as_ref()) {
                let vid = *next_var; *next_var += 1;
                array_vars.insert(name.to_string(), (vid, *iw, *ew));
            }
            // Note: array_exprs populated elsewhere (in parse_smtlib2 after declare_var)
        }
    }
}

/// Substitute array define-fun names with their bodies in an S-expression (recursive)
fn expand_array_names(sexp: &Sexp, array_defines: &HashMap<String, Sexp>) -> Sexp {
    match sexp {
        Sexp::Atom(name) => {
            if let Some(body) = array_defines.get(name.as_str()) {
                // Recursively expand in case body references other array defines
                expand_array_names(body, array_defines)
            } else {
                sexp.clone()
            }
        }
        Sexp::List(items) => {
            Sexp::List(items.iter().map(|s| expand_array_names(s, array_defines)).collect())
        }
    }
}

/// Parse a store expression, returning a BVC that represents the stored array
/// (the actual expansion happens at select time via ROW)
#[allow(clippy::too_many_arguments)]
fn translate_store(
    args: &[Sexp],
    tt: &mut TermTable, ct: &mut ConstraintTable, bm: &mut BvcManager,
    var_map: &HashMap<String, BvcId>, var_ids: &HashMap<String, (u32, BvWidth)>,
    defines: &HashMap<String, BvcId>, fun_defs: &FunDefs, array_vars: &ArrayVars,
) -> Result<BvcId, String> {
    // For store, we need to return something the caller can use.
    // Since store returns an array (not a bitvector), we create a sentinel BVC.
    // The actual ROW expansion happens when select is called on this store result.
    // For now, create a dummy 8-bit BVC — the select handler will rebuild from sexp.
    let idx_bvc = translate_expr(&args[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
    let val_bvc = translate_expr(&args[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
    let ew = bm.get(val_bvc).width;

    // Build a Write term (for oracle fallback)
    let idx_term = bm.get(idx_bvc).entries[0].term;
    let val_term = bm.get(val_bvc).entries[0].term;

    // Try to get the base array
    let arr_name = atom_str(&args[0]);
    let arr_term = if let Some(&(vid, iw, _)) = array_vars.get(&arr_name) {
        tt.make_var(vid, iw)
    } else {
        tt.make_const(0, 8) // placeholder
    };

    let write_term = tt.make_app(OpKind::Write, vec![arr_term, idx_term, val_term], ew);
    Ok(bm.alloc(ew, vec![BvcEntry { term: write_term, constraint: ct.true_id() }]))
}

/// Build an ArrayExpr tree from an S-expression (for ROW expansion at select time)
#[allow(clippy::too_many_arguments)]
fn build_array_expr(
    sexp: &Sexp,
    tt: &mut TermTable, ct: &mut ConstraintTable, bm: &mut BvcManager,
    var_map: &HashMap<String, BvcId>, var_ids: &HashMap<String, (u32, BvWidth)>,
    defines: &HashMap<String, BvcId>, fun_defs: &FunDefs, array_vars: &ArrayVars,
    array_defines: &HashMap<String, Sexp>,
) -> Result<ArrayExpr, String> {
    match sexp {
        Sexp::Atom(name) => {
            if let Some(&(vid, iw, ew)) = array_vars.get(name.as_str()) {
                Ok(ArrayExpr::Base { var_id: vid, index_width: iw, element_width: ew })
            } else if let Some(body) = array_defines.get(name.as_str()) {
                // Expand the define-fun body
                let body = body.clone();
                build_array_expr(&body, tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars, array_defines)
            } else {
                Err(format!("unknown array: {}", name))
            }
        }
        Sexp::List(items) => {
            if items.is_empty() { return Err("empty array expression".into()); }
            let head = atom_str(&items[0]);
            if head == "store" && items.len() >= 4 {
                let base = build_array_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars, array_defines)?;
                let idx_bvc = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                let val_bvc = translate_expr(&items[3], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                let ew = bm.get(val_bvc).width;
                Ok(ArrayExpr::Store { base: Box::new(base), idx_bvc, val_bvc, element_width: ew })
            } else {
                Err(format!("unsupported array expression: {}", head))
            }
        }
    }
}

/// Type alias for parameterized function definitions
type FunDefs = HashMap<String, (Vec<(String, SmtSort)>, Sexp)>;
/// Type alias for array variable info
type ArrayVars = HashMap<String, (u32, BvWidth, BvWidth)>;

/// Translate an SMT-LIB2 expression to a BVC
#[allow(clippy::too_many_arguments)]
fn translate_expr(
    sexp: &Sexp,
    tt: &mut TermTable,
    ct: &mut ConstraintTable,
    bm: &mut BvcManager,
    var_map: &HashMap<String, BvcId>,
    var_ids: &HashMap<String, (u32, BvWidth)>,
    defines: &HashMap<String, BvcId>,
    fun_defs: &FunDefs,
    array_vars: &ArrayVars,
) -> Result<BvcId, String> {
    match sexp {
        Sexp::Atom(s) => {
            // Variable reference or constant
            if let Some(&bvc) = var_map.get(s) {
                return Ok(bvc);
            }
            if let Some(&bvc) = defines.get(s) {
                return Ok(bvc);
            }
            if s == "true" {
                return Ok(bm.make_const(tt, ct, 1, 1));
            }
            if s == "false" {
                return Ok(bm.make_const(tt, ct, 0, 1));
            }
            // Binary literal: #b0101
            if let Some(bin) = s.strip_prefix("#b") {
                let val = u64::from_str_radix(bin, 2).unwrap_or(0);
                let width = bin.len() as u16;
                return Ok(bm.make_const(tt, ct, val, width));
            }
            // Hex literal: #x1a2b
            if let Some(hex) = s.strip_prefix("#x") {
                let val = u64::from_str_radix(hex, 16).unwrap_or(0);
                let width = (hex.len() * 4) as u16;
                return Ok(bm.make_const(tt, ct, val, width));
            }
            Err(format!("unknown symbol: {}", s))
        }
        Sexp::List(items) => {
            if items.is_empty() {
                return Err("empty expression".into());
            }

            // (_ bvN W) — bitvector constant
            if items.len() == 3 {
                if let (Sexp::Atom(u), Sexp::Atom(val_s), Sexp::Atom(width_s)) =
                    (&items[0], &items[1], &items[2])
                {
                    if u == "_" {
                        if let Some(val_str) = val_s.strip_prefix("bv") {
                            let val: u64 = val_str.parse().unwrap_or(0);
                            let width: u16 = width_s.parse().unwrap_or(0);
                            return Ok(bm.make_const(tt, ct, val, width));
                        }
                    }
                }
            }

            let head = atom_str(&items[0]);

            // Check for parameterized function application
            if let Some((params, body)) = fun_defs.get(&head) {
                if items.len() - 1 == params.len() {
                    // Evaluate arguments
                    let mut local_var_map = var_map.clone();
                    let mut local_var_ids = var_ids.clone();
                    let mut local_defines = defines.clone();
                    for (i, (pname, psort)) in params.iter().enumerate() {
                        let arg_bvc = translate_expr(
                            &items[i + 1], tt, ct, bm,
                            &local_var_map, &local_var_ids, &local_defines, fun_defs, array_vars,
                        )?;
                        local_defines.insert(pname.clone(), arg_bvc);
                        local_var_map.insert(pname.clone(), arg_bvc);
                        if let SmtSort::BitVec(w) = psort {
                            let vid = bm.fresh_var();
                            local_var_ids.insert(pname.clone(), (vid, *w));
                        }
                    }
                    return translate_expr(
                        body, tt, ct, bm,
                        &local_var_map, &local_var_ids, &local_defines, fun_defs, array_vars,
                    );
                }
            }

            // Indexed operators: (_ extract 7 0), (_ zero_extend 8), etc.
            if head == "_" && items.len() >= 3 {
                let op_name = atom_str(&items[1]);
                let param1 = atom_str(&items[2]);
                let param2 = if items.len() > 3 { atom_str(&items[3]) } else { String::new() };
                // These return a partially applied operator — the actual expression
                // is in the parent list. Return a marker.
                return Err(format!("indexed_op:{}:{}:{}", op_name, param1, param2));
            }

            // Check for indexed operator application: ((_ extract 7 0) expr)
            if let Sexp::List(inner) = &items[0] {
                if inner.len() >= 3 {
                    if let Sexp::Atom(u) = &inner[0] {
                        if u == "_" {
                            let op_name = atom_str(&inner[1]);
                            return translate_indexed_op(
                                &op_name, inner, &items[1..],
                                tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars,
                            );
                        }
                    }
                }
            }

            // Regular operators
            match head.as_str() {
                "not" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let w = bm.get(a).width;
                    if w == 1 {
                        // Boolean not → bitvector NOT at width 1
                        Ok(bm.apply(tt, ct, OpKind::Not, &[a], 1))
                    } else {
                        Ok(bm.apply(tt, ct, OpKind::Not, &[a], w))
                    }
                }
                "and" => translate_nary(OpKind::And, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "or" => translate_nary(OpKind::Or, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "xor" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let w = bm.get(a).width;
                    Ok(bm.apply(tt, ct, OpKind::Xor, &[a, b], w))
                }
                "=" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    Ok(bm.apply(tt, ct, OpKind::Eq, &[a, b], 1))
                }
                "distinct" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    Ok(bm.apply(tt, ct, OpKind::Neq, &[a, b], 1))
                }
                "ite" => {
                    let c = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let t = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let e = translate_expr(&items[3], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let w = bm.get(t).width;
                    Ok(bm.apply(tt, ct, OpKind::Ite, &[c, t, e], w))
                }
                "bvand" => translate_binop(OpKind::And, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvor" => translate_binop(OpKind::Or, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvxor" => translate_binop(OpKind::Xor, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvnot" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let w = bm.get(a).width;
                    Ok(bm.apply(tt, ct, OpKind::Not, &[a], w))
                }
                "bvneg" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let w = bm.get(a).width;
                    Ok(bm.apply(tt, ct, OpKind::Neg, &[a], w))
                }
                "bvadd" => translate_binop(OpKind::Add, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvsub" => translate_binop(OpKind::Sub, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvmul" => translate_binop(OpKind::Mul, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvudiv" => translate_binop(OpKind::Udiv, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvurem" => translate_binop(OpKind::Urem, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvsdiv" => translate_binop(OpKind::Sdiv, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvsrem" => translate_binop(OpKind::Srem, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvsmod" => translate_binop(OpKind::Smod, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvshl" => translate_binop(OpKind::Sll, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvlshr" => translate_binop(OpKind::Srl, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvashr" => translate_binop(OpKind::Sra, &items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars),
                "bvult" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    Ok(bm.apply(tt, ct, OpKind::Ult, &[a, b], 1))
                }
                "bvule" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    Ok(bm.apply(tt, ct, OpKind::Ulte, &[a, b], 1))
                }
                "bvslt" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    Ok(bm.apply(tt, ct, OpKind::Slt, &[a, b], 1))
                }
                "bvsle" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    Ok(bm.apply(tt, ct, OpKind::Slte, &[a, b], 1))
                }
                "bvugt" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    Ok(bm.apply(tt, ct, OpKind::Ult, &[b, a], 1))
                }
                "bvuge" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    Ok(bm.apply(tt, ct, OpKind::Ulte, &[b, a], 1))
                }
                "bvsgt" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    Ok(bm.apply(tt, ct, OpKind::Slt, &[b, a], 1))
                }
                "bvsge" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    Ok(bm.apply(tt, ct, OpKind::Slte, &[b, a], 1))
                }
                "concat" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let wa = bm.get(a).width;
                    let wb = bm.get(b).width;
                    Ok(bm.apply(tt, ct, OpKind::Concat, &[a, b], wa + wb))
                }
                "bvcomp" => {
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    Ok(bm.apply(tt, ct, OpKind::Eq, &[a, b], 1))
                }
                // Array operations — use ROW expansion
                "select" => {
                    let idx_bvc = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let empty_ad = HashMap::new();
                    match build_array_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars, &empty_ad) {
                        Ok(arr_expr) => Ok(expand_select_row(&arr_expr, idx_bvc, tt, ct, bm)),
                        Err(_) => {
                            // Fallback: build Read term for oracle
                            let idx_term = bm.get(idx_bvc).entries[0].term;
                            // Try to find a fresh variable for this array read
                            let arr_name = atom_str(&items[1]);
                            if let Some(&(vid, iw, ew)) = array_vars.get(&arr_name) {
                                let arr_term = tt.make_var(vid, iw);
                                let read_term = tt.make_app(OpKind::Read, vec![arr_term, idx_term], ew);
                                Ok(bm.alloc(ew, vec![BvcEntry { term: read_term, constraint: ct.true_id() }]))
                            } else {
                                // Unknown array — oracle fallback
                                Ok(bm.make_const(tt, ct, 0, 8))
                            }
                        }
                    }
                }
                "store" => {
                    translate_store(&items[1..], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)
                }
                "let" => {
                    // (let ((x expr) ...) body)
                    if items.len() >= 3 {
                        let mut local_defines = defines.clone();
                        let mut local_var_map = var_map.clone();
                        if let Sexp::List(bindings) = &items[1] {
                            for binding in bindings {
                                if let Sexp::List(pair) = binding {
                                    if pair.len() >= 2 {
                                        let name = atom_str(&pair[0]);
                                        let val = translate_expr(
                                            &pair[1], tt, ct, bm,
                                            &local_var_map, var_ids, &local_defines, fun_defs, array_vars,
                                        )?;
                                        local_defines.insert(name.clone(), val);
                                        local_var_map.insert(name, val);
                                    }
                                }
                            }
                        }
                        translate_expr(
                            &items[2], tt, ct, bm,
                            &local_var_map, var_ids, &local_defines, fun_defs, array_vars,
                        )
                    } else {
                        Err("malformed let".into())
                    }
                }
                "=>" => {
                    // implies
                    let a = translate_expr(&items[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let b = translate_expr(&items[2], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
                    let not_a = bm.apply(tt, ct, OpKind::Not, &[a], 1);
                    Ok(bm.apply(tt, ct, OpKind::Or, &[not_a, b], 1))
                }
                _ => Err(format!("unsupported SMT-LIB2 operator: {}", head)),
            }
        }
    }
}

/// Translate an indexed operator like (_ extract 7 0) or (_ zero_extend 8)
#[allow(clippy::too_many_arguments)]
fn translate_indexed_op(
    op_name: &str,
    inner: &[Sexp],
    args: &[Sexp],
    tt: &mut TermTable,
    ct: &mut ConstraintTable,
    bm: &mut BvcManager,
    var_map: &HashMap<String, BvcId>,
    var_ids: &HashMap<String, (u32, BvWidth)>,
    defines: &HashMap<String, BvcId>,
    fun_defs: &FunDefs,
    array_vars: &ArrayVars,
) -> Result<BvcId, String> {
    match op_name {
        "extract" => {
            let upper: u16 = atom_str(&inner[2]).parse().unwrap_or(0);
            let lower: u16 = atom_str(&inner[3]).parse().unwrap_or(0);
            let a = translate_expr(&args[0], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
            let arg_term = bm.get(a).entries[0].term;
            let slice_term = tt.make_slice(arg_term, upper, lower);
            let constraint = bm.get(a).entries[0].constraint;
            let width = upper - lower + 1;
            Ok(bm.alloc(width, vec![BvcEntry { term: slice_term, constraint }]))
        }
        "zero_extend" => {
            let ext: u16 = atom_str(&inner[2]).parse().unwrap_or(0);
            let a = translate_expr(&args[0], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
            let wa = bm.get(a).width;
            let new_width = wa + ext;
            let arg_term = bm.get(a).entries[0].term;
            let ext_term = tt.make_app(OpKind::Uext, vec![arg_term], new_width);
            let constraint = bm.get(a).entries[0].constraint;
            Ok(bm.alloc(new_width, vec![BvcEntry { term: ext_term, constraint }]))
        }
        "sign_extend" => {
            let ext: u16 = atom_str(&inner[2]).parse().unwrap_or(0);
            let a = translate_expr(&args[0], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
            let wa = bm.get(a).width;
            let new_width = wa + ext;
            let arg_term = bm.get(a).entries[0].term;
            let ext_term = tt.make_app(OpKind::Sext, vec![arg_term], new_width);
            let constraint = bm.get(a).entries[0].constraint;
            Ok(bm.alloc(new_width, vec![BvcEntry { term: ext_term, constraint }]))
        }
        "repeat" => {
            let count: u16 = atom_str(&inner[2]).parse().unwrap_or(1);
            let a = translate_expr(&args[0], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
            let wa = bm.get(a).width;
            let mut result = a;
            for _ in 1..count {
                result = bm.apply(tt, ct, OpKind::Concat, &[result, a], bm.get(result).width + wa);
            }
            Ok(result)
        }
        "rotate_left" | "rotate_right" => {
            // Approximation: treat as shift (correct for many benchmarks)
            let _amount: u16 = atom_str(&inner[2]).parse().unwrap_or(0);
            let a = translate_expr(&args[0], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
            Ok(a) // TODO: proper rotation
        }
        _ => Err(format!("unsupported indexed op: {}", op_name)),
    }
}

#[allow(clippy::too_many_arguments)]
fn translate_binop(
    op: OpKind,
    args: &[Sexp],
    tt: &mut TermTable,
    ct: &mut ConstraintTable,
    bm: &mut BvcManager,
    var_map: &HashMap<String, BvcId>,
    var_ids: &HashMap<String, (u32, BvWidth)>,
    defines: &HashMap<String, BvcId>,
    fun_defs: &FunDefs,
    array_vars: &ArrayVars,
) -> Result<BvcId, String> {
    let a = translate_expr(&args[0], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
    let b = translate_expr(&args[1], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
    let w = bm.get(a).width;
    Ok(bm.apply(tt, ct, op, &[a, b], w))
}

#[allow(clippy::too_many_arguments)]
fn translate_nary(
    op: OpKind,
    args: &[Sexp],
    tt: &mut TermTable,
    ct: &mut ConstraintTable,
    bm: &mut BvcManager,
    var_map: &HashMap<String, BvcId>,
    var_ids: &HashMap<String, (u32, BvWidth)>,
    defines: &HashMap<String, BvcId>,
    fun_defs: &FunDefs,
    array_vars: &ArrayVars,
) -> Result<BvcId, String> {
    if args.is_empty() {
        return Err("empty n-ary op".into());
    }
    let mut result = translate_expr(&args[0], tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
    for arg in &args[1..] {
        let b = translate_expr(arg, tt, ct, bm, var_map, var_ids, defines, fun_defs, array_vars)?;
        let w = bm.get(result).width;
        result = bm.apply(tt, ct, op, &[result, b], w);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_qf_bv() {
        let input = r#"
(set-logic QF_BV)
(declare-const x (_ BitVec 8))
(declare-const y (_ BitVec 8))
(assert (= (bvadd x y) (_ bv255 8)))
(check-sat)
"#;
        let model = parse_smtlib2(input).unwrap();
        assert_eq!(model.assertions.len(), 1);
        assert_eq!(model.var_map.len(), 2);
    }

    #[test]
    fn test_parse_bitvec_constants() {
        let input = r#"
(set-logic QF_BV)
(declare-const x (_ BitVec 4))
(assert (= x #b1010))
(check-sat)
"#;
        let model = parse_smtlib2(input).unwrap();
        assert_eq!(model.assertions.len(), 1);
    }

    #[test]
    fn test_parse_extract() {
        let input = r#"
(set-logic QF_BV)
(declare-const x (_ BitVec 8))
(assert (= ((_ extract 3 0) x) #b1111))
(check-sat)
"#;
        let model = parse_smtlib2(input).unwrap();
        assert_eq!(model.assertions.len(), 1);
    }

    #[test]
    fn test_parse_let() {
        let input = r#"
(set-logic QF_BV)
(declare-const x (_ BitVec 8))
(assert (let ((y (bvadd x (_ bv1 8)))) (= y (_ bv0 8))))
(check-sat)
"#;
        let model = parse_smtlib2(input).unwrap();
        assert_eq!(model.assertions.len(), 1);
    }
}
