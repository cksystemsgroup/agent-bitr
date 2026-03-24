(set-logic QF_BV)
(set-info :status sat)

; Nested let bindings — a pattern common in SMT-COMP benchmarks from
; program verification tools that introduce many temporaries.

(declare-const a (_ BitVec 8))
(declare-const b (_ BitVec 8))

(assert
  (let ((sum (bvadd a b)))
    (let ((doubled (bvadd sum sum)))
      (let ((masked (bvand doubled #xFF)))
        (let ((final (bvadd masked #x01)))
          (= final #x55))))))

; Additional constraint to narrow the search
(assert (bvugt a #x10))
(assert (bvugt b #x05))

(check-sat)
(exit)
