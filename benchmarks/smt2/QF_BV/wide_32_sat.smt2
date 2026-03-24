(set-logic QF_BV)
(set-info :status sat)

; 32-bit arithmetic chain: models a hash-like mixing function.
; Find x such that after mixing, the result equals a target.

(declare-const x (_ BitVec 32))
(declare-const h1 (_ BitVec 32))
(declare-const h2 (_ BitVec 32))
(declare-const h3 (_ BitVec 32))

; Step 1: h1 = x XOR (x >> 16)
(assert (= h1 (bvxor x (bvlshr x (_ bv16 32)))))

; Step 2: h2 = h1 * 0x45d9f3b
(assert (= h2 (bvmul h1 #x045d9f3b)))

; Step 3: h3 = h2 XOR (h2 >> 16)
(assert (= h3 (bvxor h2 (bvlshr h2 (_ bv16 32)))))

; Target: h3 must have specific low byte
(assert (= ((_ extract 7 0) h3) #xAB))

; x must be nonzero
(assert (not (= x (_ bv0 32))))

(check-sat)
(exit)
