; Diagnostic only: pinned dialect 0.2.10 §11.6 effect-console-r. Not hello.c.
@msg = private unnamed_addr constant [3 x i8] c"hi\0A"
declare void @__bennett_effect_console_write(ptr, i64)
define i32 @main() {
  call void @__bennett_effect_console_write(ptr @msg, i64 3)
  ret i32 0
}
