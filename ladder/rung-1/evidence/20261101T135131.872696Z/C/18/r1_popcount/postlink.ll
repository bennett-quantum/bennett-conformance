; ModuleID = '/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-1/20261101T135131.872696Z/C/18/r1_popcount/linked.bc'
source_filename = "llvm-link"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

@r1_lookup.table = internal unnamed_addr constant [8 x i8] c"\03\F9\0B\00\7F\80*\FF", align 1

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef signext i8 @r1_sat_add(i8 noundef signext %0, i8 noundef signext %1) local_unnamed_addr #0 {
  %3 = tail call i8 @llvm.sadd.sat.i8(i8 %0, i8 %1)
  ret i8 %3
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.sadd.sat.i8(i8, i8) #1

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local signext i8 @r1_popcount(i8 noundef signext %0) local_unnamed_addr #0 {
  %2 = tail call i8 @llvm.ctpop.i8(i8 %0), !range !7
  ret i8 %2
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.ctpop.i8(i8) #1

; Function Attrs: nofree norecurse nosync nounwind memory(none) uwtable
define dso_local signext i8 @r1_gcd(i8 noundef signext %0, i8 noundef signext %1) local_unnamed_addr #2 {
  %3 = tail call i8 @llvm.abs.i8(i8 %0, i1 false)
  %4 = icmp eq i8 %1, 0
  br i1 %4, label %16, label %5

5:                                                ; preds = %2
  %6 = zext i8 %3 to i32
  %7 = tail call i8 @llvm.abs.i8(i8 %1, i1 false)
  %8 = zext i8 %7 to i32
  br label %9

9:                                                ; preds = %9, %5
  %10 = phi i32 [ %11, %9 ], [ %6, %5 ]
  %11 = phi i32 [ %12, %9 ], [ %8, %5 ]
  %12 = urem i32 %10, %11
  %13 = icmp eq i32 %12, 0
  br i1 %13, label %14, label %9, !llvm.loop !8

14:                                               ; preds = %9
  %.lcssa = phi i32 [ %11, %9 ]
  %15 = trunc i32 %.lcssa to i8
  br label %16

16:                                               ; preds = %14, %2
  %17 = phi i8 [ %3, %2 ], [ %15, %14 ]
  ret i8 %17
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.abs.i8(i8, i1 immarg) #1

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local signext i8 @r1_sign_abs(i8 noundef signext %0) local_unnamed_addr #0 {
  %2 = icmp eq i8 %0, -128
  br i1 %2, label %11, label %3

3:                                                ; preds = %1
  %4 = icmp slt i8 %0, 0
  br i1 %4, label %5, label %7

5:                                                ; preds = %3
  %6 = sub nsw i8 0, %0
  br label %11

7:                                                ; preds = %3
  %8 = icmp eq i8 %0, 0
  %9 = lshr i8 %0, 1
  %10 = select i1 %8, i8 -1, i8 %9
  br label %11

11:                                               ; preds = %7, %5, %1
  %12 = phi i8 [ %6, %5 ], [ %10, %7 ], [ 127, %1 ]
  ret i8 %12
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local signext i8 @r1_lookup(i8 noundef signext %0) local_unnamed_addr #0 {
  %2 = and i8 %0, 7
  %3 = zext nneg i8 %2 to i64
  %4 = getelementptr inbounds [8 x i8], ptr @r1_lookup.table, i64 0, i64 %3
  %5 = load i8, ptr %4, align 1, !tbaa !10
  ret i8 %5
}

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #2 = { nofree norecurse nosync nounwind memory(none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }

!llvm.ident = !{!0}
!llvm.module.flags = !{!1, !2, !3, !4, !5, !6}

!0 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
!1 = !{i32 1, !"wchar_size", i32 4}
!2 = !{i32 8, !"PIC Level", i32 2}
!3 = !{i32 7, !"PIE Level", i32 2}
!4 = !{i32 7, !"uwtable", i32 2}
!5 = !{i32 1, !"ThinLTO", i32 0}
!6 = !{i32 1, !"EnableSplitLTOUnit", i32 1}
!7 = !{i8 0, i8 9}
!8 = distinct !{!8, !9}
!9 = !{!"llvm.loop.mustprogress"}
!10 = !{!11, !11, i64 0}
!11 = !{!"omnipotent char", !12, i64 0}
!12 = !{!"Simple C/C++ TBAA"}
