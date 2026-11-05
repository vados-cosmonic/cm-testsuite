;; Intent: integer addition returns the expected value for two small operands.
(module
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add))
;; The assertion checks the exported function's result, not just module validity.
(assert_return (invoke "add" (i32.const 2) (i32.const 3)) (i32.const 5))
