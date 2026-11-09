; Diagnostic only: pinned dialect 0.2.10 §11.6 effect-console-r. Not hello.c.
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-musl"
@msg = private unnamed_addr constant [3 x i8] c"hi\0A"
declare void @__bennett_effect_console_write(ptr, i64)
define i32 @main() {
  call void @__bennett_effect_console_write(ptr @msg, i64 3)
  ret i32 0
}
