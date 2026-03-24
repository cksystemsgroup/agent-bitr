(set-logic QF_BV)
(set-info :status sat)

; Shift and rotate operations.
; Find x such that rotating left by 8 and shifting right by 4 yields a target.

(declare-const x (_ BitVec 16))
(declare-const y (_ BitVec 16))
(declare-const z (_ BitVec 16))

; y = rotate_left(x, 8)  — swap bytes
(assert (= y (concat ((_ extract 7 0) x) ((_ extract 15 8) x))))

; z = logical shift right y by 4
(assert (= z (bvlshr y (_ bv4 16))))

; Constrain z to a known value
(assert (= z #x0AB0))

; Also require the low nibble of x to be nonzero
(assert (not (= ((_ extract 3 0) x) #b0000)))

(check-sat)
(exit)
