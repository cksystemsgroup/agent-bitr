(set-logic QF_ABV)
(set-info :status sat)

; Two writes at different indices, read both, check values match. SAT.

(declare-fun a () (Array (_ BitVec 8) (_ BitVec 8)))

(define-fun a1 () (Array (_ BitVec 8) (_ BitVec 8))
  (store a (_ bv3 8) (_ bv10 8)))

(define-fun a2 () (Array (_ BitVec 8) (_ BitVec 8))
  (store a1 (_ bv7 8) (_ bv20 8)))

(assert (= (select a2 (_ bv3 8)) (_ bv10 8)))
(assert (= (select a2 (_ bv7 8)) (_ bv20 8)))

(check-sat)
(exit)
