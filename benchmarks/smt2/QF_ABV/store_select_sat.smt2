(set-logic QF_ABV)
(set-info :status sat)

; Write val=42 at index 1, read index 1, check == 42. SAT.

(declare-fun a () (Array (_ BitVec 8) (_ BitVec 8)))

(define-fun a1 () (Array (_ BitVec 8) (_ BitVec 8))
  (store a (_ bv1 8) (_ bv42 8)))

(assert (= (select a1 (_ bv1 8)) (_ bv42 8)))

(check-sat)
(exit)
