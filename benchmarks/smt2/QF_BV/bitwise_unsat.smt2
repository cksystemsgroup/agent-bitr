(set-logic QF_BV)
(set-info :status unsat)

; Bitwise contradiction: constraints on AND, OR, XOR that are mutually exclusive.

(declare-const a (_ BitVec 16))
(declare-const b (_ BitVec 16))
(declare-const c (_ BitVec 16))

; c = a AND b
(assert (= c (bvand a b)))

; All bits of c are 1 (so a AND b = 0xFFFF, meaning a = b = 0xFFFF)
(assert (= c #xFFFF))

; a XOR b must be nonzero
; But if a = b = 0xFFFF then a XOR b = 0x0000, contradiction.
(assert (not (= (bvxor a b) #x0000)))

(check-sat)
(exit)
