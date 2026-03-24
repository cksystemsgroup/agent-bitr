(set-logic QF_BV)
(set-info :status sat)

; Extract and concat: split a 32-bit word, swap halves, check result.
; x = 0xAABBCCDD => hi=0xAABB, lo=0xCCDD => swapped=0xCCDDAABB

(declare-const x (_ BitVec 32))
(declare-const y (_ BitVec 32))

; Extract upper and lower 16-bit halves of x
(assert (= y (concat ((_ extract 15 0) x) ((_ extract 31 16) x))))

; Constrain x to a specific value
(assert (= x #xAABBCCDD))

; The swapped result should be 0xCCDDAABB
(assert (= y #xCCDDAABB))

(check-sat)
(exit)
