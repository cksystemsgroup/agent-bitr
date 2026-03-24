(set-logic QF_BV)
(set-info :status unsat)

; No 16-bit value x satisfies x*x + 1 == 0 (mod 2^16)
; In any ring Z/(2^n), -1 is not a quadratic residue for n >= 3.

(declare-const x (_ BitVec 16))

(assert (= (bvadd (bvmul x x) (_ bv1 16)) (_ bv0 16)))

(check-sat)
(exit)
