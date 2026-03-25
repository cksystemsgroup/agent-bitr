(set-logic QF_ABV)
(set-info :status unsat)

; Store at index 0, read at index 0, check stored_val != read_val. UNSAT.
; Read-after-write at the same index always returns the written value.

(declare-fun a () (Array (_ BitVec 8) (_ BitVec 8)))
(declare-fun v () (_ BitVec 8))

(define-fun a1 () (Array (_ BitVec 8) (_ BitVec 8))
  (store a (_ bv0 8) v))

(assert (not (= (select a1 (_ bv0 8)) v)))

(check-sat)
(exit)
