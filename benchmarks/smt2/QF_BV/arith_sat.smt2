(set-logic QF_BV)
(set-info :status sat)

; Modular inverse: find x such that x * 37 == 1 (mod 2^32)
; Solution: x = 0xDD6A7B0D (3714566925)
; Verification: 37 * 3714566925 = 137438953825 = 32 * 2^32 + 1

(declare-const x (_ BitVec 32))

(assert (= (bvmul x (_ bv37 32)) (_ bv1 32)))

(check-sat)
(exit)
