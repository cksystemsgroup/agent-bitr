(set-logic QF_BV)
(set-info :status unsat)

; Multiple assertions that progressively constrain variables.
; Models a simplified register-transfer scenario.

(declare-const r0 (_ BitVec 32))
(declare-const r1 (_ BitVec 32))
(declare-const r2 (_ BitVec 32))
(declare-const r3 (_ BitVec 32))

; r0 is nonzero
(assert (not (= r0 (_ bv0 32))))

; r1 = r0 + 0x100
(assert (= r1 (bvadd r0 (_ bv256 32))))

; r2 = r1 AND 0xFFFF0000 (clear lower 16 bits)
(assert (= r2 (bvand r1 #xFFFF0000)))

; r3 = r2 OR 0x0000DEAD
(assert (= r3 (bvor r2 #x0000DEAD)))

; The low 16 bits of r3 must be 0xDEAD
(assert (= ((_ extract 15 0) r3) #xDEAD))

; r0 is less than 0x10000 (fits in 16 bits)
(assert (bvult r0 (_ bv65536 32)))

; The upper byte of r3 must be nonzero
(assert (not (= ((_ extract 31 24) r3) #x00)))

(check-sat)
(exit)
