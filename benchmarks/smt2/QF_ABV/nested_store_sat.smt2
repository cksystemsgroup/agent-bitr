(set-logic QF_ABV)
(set-info :status sat)

; Three nested stores at indices 1, 2, 3 with values 11, 22, 33.
; Read from the middle index (2) and check it equals 22. SAT.

(declare-fun a () (Array (_ BitVec 8) (_ BitVec 8)))

(define-fun a1 () (Array (_ BitVec 8) (_ BitVec 8))
  (store a (_ bv1 8) (_ bv11 8)))

(define-fun a2 () (Array (_ BitVec 8) (_ BitVec 8))
  (store a1 (_ bv2 8) (_ bv22 8)))

(define-fun a3 () (Array (_ BitVec 8) (_ BitVec 8))
  (store a2 (_ bv3 8) (_ bv33 8)))

(assert (= (select a3 (_ bv2 8)) (_ bv22 8)))

(check-sat)
(exit)
