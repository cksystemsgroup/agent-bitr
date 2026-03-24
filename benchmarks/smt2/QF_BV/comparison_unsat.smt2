(set-logic QF_BV)
(set-info :status sat)

; Unsigned/signed comparison constraints that ARE satisfiable.
; e.g., x=207, y=49: x>200, y<50, x+y=0 mod 256, x signed < 0.

(declare-const x (_ BitVec 8))
(declare-const y (_ BitVec 8))

; x is unsigned-greater-than 200
(assert (bvugt x (_ bv200 8)))

; y is unsigned-less-than 50
(assert (bvult y (_ bv50 8)))

; x + y == 0  (mod 256)  =>  y == 256 - x  =>  y >= 56 (since x <= 255)
; But y < 50, contradiction.
(assert (= (bvadd x y) (_ bv0 8)))

; Additional: x is signed-less-than 0 (i.e., x >= 128 in unsigned, which is consistent with x > 200)
(assert (bvslt x (_ bv0 8)))

(check-sat)
(exit)
