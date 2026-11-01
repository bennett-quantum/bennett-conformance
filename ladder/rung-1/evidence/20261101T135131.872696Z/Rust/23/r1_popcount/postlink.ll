; ModuleID = '/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-1/20261101T135131.872696Z/Rust/23/r1_popcount/linked.bc'
source_filename = "llvm-link"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@anon.b404e272f1e7e838529ae840e71594ab.0 = private unnamed_addr constant [8 x i8] c"\03\F9\0B\00\7F\80*\FF", align 1, !guid !0

; Function Attrs: nofree norecurse nosync nounwind nonlazybind memory(none) uwtable
define noundef range(i8 0, -127) i8 @r1_gcd(i8 noundef signext %a, i8 noundef signext %b) unnamed_addr #0 personality ptr @rust_eh_personality !guid !13 {
start:
  %_9.sroa.0.0 = tail call i8 @llvm.abs.i8(i8 %a, i1 false)
  %0 = icmp eq i8 %b, 0
  br i1 %0, label %bb4, label %bb3.preheader

bb3.preheader:                                    ; preds = %start
  %_11.sroa.0.0 = tail call i8 @llvm.abs.i8(i8 %b, i1 false)
  br label %bb3

bb4:                                              ; preds = %bb3, %start
  %x.sroa.0.0.lcssa = phi i8 [ %_9.sroa.0.0, %start ], [ %y.sroa.0.04, %bb3 ]
  ret i8 %x.sroa.0.0.lcssa

bb3:                                              ; preds = %bb3, %bb3.preheader
  %x.sroa.0.05 = phi i8 [ %y.sroa.0.04, %bb3 ], [ %_9.sroa.0.0, %bb3.preheader ]
  %y.sroa.0.04 = phi i8 [ %r, %bb3 ], [ %_11.sroa.0.0, %bb3.preheader ]
  %r = urem i8 %x.sroa.0.05, %y.sroa.0.04
  %1 = icmp eq i8 %r, 0
  br i1 %1, label %bb4, label %bb3
}

; Function Attrs: nonlazybind
declare i32 @rust_eh_personality(...) unnamed_addr #1

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.abs.i8(i8, i1 immarg) #2

; Function Attrs: mustprogress nofree norecurse nosync nounwind nonlazybind willreturn memory(none) uwtable
define noundef i8 @r1_lookup(i8 noundef signext %x) unnamed_addr #3 personality ptr @rust_eh_personality !guid !14 {
start:
  %_4 = and i8 %x, 7
  %_3 = zext nneg i8 %_4 to i64
  %0 = getelementptr inbounds nuw i8, ptr @anon.b404e272f1e7e838529ae840e71594ab.0, i64 %_3
  %_0 = load i8, ptr %0, align 1, !noundef !15
  ret i8 %_0
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind nonlazybind willreturn memory(none) uwtable
define noundef range(i8 0, 9) i8 @r1_popcount(i8 noundef signext %x) unnamed_addr #3 !guid !16 {
start:
  %0 = tail call range(i8 0, 9) i8 @llvm.ctpop.i8(i8 %x)
  ret i8 %0
}

; Function Attrs: mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.ctpop.i8(i8) #4

; Function Attrs: mustprogress nofree norecurse nosync nounwind nonlazybind willreturn memory(none) uwtable
define noundef i8 @r1_sat_add(i8 noundef signext %a, i8 noundef signext %b) unnamed_addr #3 !guid !17 {
start:
  %_0 = tail call i8 @llvm.sadd.sat.i8(i8 %a, i8 %b)
  ret i8 %_0
}

; Function Attrs: mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.sadd.sat.i8(i8, i8) #4

; Function Attrs: mustprogress nofree norecurse nosync nounwind nonlazybind willreturn memory(none) uwtable
define noundef range(i8 -1, -128) i8 @r1_sign_abs(i8 noundef signext %x) unnamed_addr #3 !guid !18 {
start:
  %0 = icmp eq i8 %x, -128
  br i1 %0, label %bb9, label %bb2

bb2:                                              ; preds = %start
  %_2 = icmp slt i8 %x, 0
  br i1 %_2, label %bb3, label %bb4

bb9:                                              ; preds = %bb3, %bb4, %start
  %_0.sroa.0.0 = phi i8 [ %spec.select, %bb4 ], [ %3, %bb3 ], [ 127, %start ]
  ret i8 %_0.sroa.0.0

bb4:                                              ; preds = %bb2
  %1 = icmp eq i8 %x, 0
  %2 = lshr i8 %x, 1
  %spec.select = select i1 %1, i8 -1, i8 %2
  br label %bb9

bb3:                                              ; preds = %bb2
  %3 = sub nsw i8 0, %x
  br label %bb9
}

attributes #0 = { nofree norecurse nosync nounwind nonlazybind memory(none) uwtable "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #1 = { nonlazybind "target-cpu"="x86-64" }
attributes #2 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #3 = { mustprogress nofree norecurse nosync nounwind nonlazybind willreturn memory(none) uwtable "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #4 = { mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none) }

!llvm.ident = !{!1}
!llvm.module.flags = !{!2, !3, !4, !5, !6, !7}
!llvm.errno.tbaa = !{!8}

!0 = !{i64 3644336709149614130}
!1 = !{!"rustc version 1.99.1 (58cf8f9c3 2026-10-13)"}
!2 = !{i32 8, !"PIC Level", i32 2}
!3 = !{i32 2, !"RtLibUseGOT", i32 1}
!4 = !{i32 7, !"uwtable", i32 2}
!5 = !{i32 7, !"PIE Level", i32 2}
!6 = !{i32 1, !"ThinLTO", i32 0}
!7 = !{i32 1, !"EnableSplitLTOUnit", i32 1}
!8 = !{!9, !10, i64 0}
!9 = !{!"__libc_errno", !10, i64 0}
!10 = !{!"int", !11, i64 0}
!11 = !{!"omnipotent char", !12, i64 0}
!12 = !{!"Simple C/C++ TBAA"}
!13 = !{i64 2662114883697144272}
!14 = !{i64 -8720007271138850881}
!15 = !{}
!16 = !{i64 8389742813133258033}
!17 = !{i64 8695154793211451450}
!18 = !{i64 4502010609203622919}
