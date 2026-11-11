;; Integer multiplication returns the product of its operands.
(module
  (func (export "multiply") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.mul))

(assert_return (invoke "multiply" (i32.const 6) (i32.const 7)) (i32.const 42))
