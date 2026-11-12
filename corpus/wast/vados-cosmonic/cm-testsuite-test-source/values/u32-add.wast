;; A lifted core function returns the wrapped 32-bit sum of its operands.
;; The second assertion checks wrap-around, which a host that widened the
;; operands before adding would get wrong.
(component
  (core module $m
    (func (export "add") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      i32.add))
  (core instance $i (instantiate $m))
  (func (export "add") (param "a" u32) (param "b" u32) (result u32)
    (canon lift (core func $i "add"))))

(assert_return (invoke "add" (u32.const 40) (u32.const 2)) (u32.const 42))
(assert_return (invoke "add" (u32.const 4294967295) (u32.const 1)) (u32.const 0))
