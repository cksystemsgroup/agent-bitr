(set-logic QF_ABV)
(set-info :status sat)

; Write at index i, read at index j. If i==j then values must match. SAT.

(declare-fun a () (Array (_ BitVec 8) (_ BitVec 8)))
(declare-fun i () (_ BitVec 8))
(declare-fun j () (_ BitVec 8))
(declare-fun v () (_ BitVec 8))

(define-fun a1 () (Array (_ BitVec 8) (_ BitVec 8))
  (store a i v))

; If i == j, then reading j from the updated array must yield v.
(assert (=> (= i j) (= (select a1 j) v)))

; Force i == j to make the scenario interesting.
(assert (= i j))

(check-sat)
(exit)
