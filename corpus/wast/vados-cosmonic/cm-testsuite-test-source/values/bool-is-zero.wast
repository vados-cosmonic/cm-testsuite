;; A lifted core function that returns 0 or 1 is observed as a `bool`.
;; Both outcomes are asserted, so a host that maps every result to the same
;; boolean fails one of them.
(component
  (core module $m
    (func (export "is-zero") (param i32) (result i32)
      local.get 0
      i32.eqz))
  (core instance $i (instantiate $m))
  (func (export "is-zero") (param "x" u32) (result bool)
    (canon lift (core func $i "is-zero"))))

(assert_return (invoke "is-zero" (u32.const 0)) (bool.const true))
(assert_return (invoke "is-zero" (u32.const 7)) (bool.const false))
