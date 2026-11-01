; ModuleID = '/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-1/20261101T135131.872696Z/Julia/23/r1_gcd/linked.bc'
source_filename = "llvm-link"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@jl_boxed_int8_cache = external local_unnamed_addr constant [256 x ptr]

; Function Attrs: norecurse nounwind
define swiftcc range(i8 0, -127) i8 @julia_r1_gcd_225(ptr nofree nonnull readonly swiftself captures(none) "gcstack" %pgcstack_arg, i8 signext %"a::Int8", i8 signext %"b::Int8") local_unnamed_addr #0 !dbg !13 {
top:
  %ptls_field = getelementptr inbounds nuw i8, ptr %pgcstack_arg, i64 16
  %ptls_load = load ptr, ptr %ptls_field, align 8, !tbaa !24
  %0 = getelementptr inbounds nuw i8, ptr %ptls_load, i64 16
  %safepoint = load atomic ptr, ptr %0 monotonic, align 8, !tbaa !28, !invariant.load !19
  fence syncscope("singlethread") seq_cst
  %1 = load volatile i64, ptr %safepoint, align 8
  fence syncscope("singlethread") seq_cst
    #dbg_value(i8 %"a::Int8", !22, !DIExpression(), !30)
    #dbg_value(i8 %"b::Int8", !23, !DIExpression(), !30)
  %2 = tail call i8 @llvm.abs.i8(i8 %"a::Int8", i1 false), !dbg !31
  %3 = icmp eq i8 %"b::Int8", 0, !dbg !38
  br i1 %3, label %L13, label %L11.preheader, !dbg !45

L11.preheader:                                    ; preds = %top
  %4 = zext i8 %2 to i16, !dbg !31
  %5 = tail call i8 @llvm.abs.i8(i8 %"b::Int8", i1 false), !dbg !31
  %6 = zext i8 %5 to i16, !dbg !31
  br label %after_srem, !dbg !46

L13.loopexit:                                     ; preds = %after_srem
  %value_phi2.lcssa = phi i16 [ %value_phi2, %after_srem ]
  %7 = trunc nuw i16 %value_phi2.lcssa to i8, !dbg !49
  br label %L13, !dbg !49

L13:                                              ; preds = %L13.loopexit, %top
  %value_phi1.lcssa = phi i8 [ %2, %top ], [ %7, %L13.loopexit ]
  ret i8 %value_phi1.lcssa, !dbg !50

after_srem:                                       ; preds = %after_srem, %L11.preheader
  %value_phi13 = phi i16 [ %value_phi2, %after_srem ], [ %4, %L11.preheader ]
  %value_phi2 = phi i16 [ %8, %after_srem ], [ %6, %L11.preheader ]
  %8 = urem i16 %value_phi13, %value_phi2, !dbg !46
  %9 = icmp eq i16 %8, 0, !dbg !38
  br i1 %9, label %L13.loopexit, label %after_srem, !dbg !45
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.abs.i8(i8, i1 immarg) #1

; Function Attrs: noinline optnone
define nonnull ptr @jfptr_r1_gcd_226(ptr %"function::Core.Function", ptr noalias noundef readonly captures(none) %"args::Any[]", i32 %"nargs::UInt32") local_unnamed_addr #2 {
top:
  %thread_ptr = call ptr asm "movq %fs:0, $0", "=r"()
  %tls_ppgcstack = getelementptr inbounds i8, ptr %thread_ptr, i64 -8
  %tls_pgcstack = load ptr, ptr %tls_ppgcstack, align 8
  %0 = load ptr, ptr %"args::Any[]", align 8, !tbaa !28, !invariant.load !19, !alias.scope !51, !noalias !54, !nonnull !19, !dereferenceable !59, !align !59
  %1 = getelementptr inbounds nuw i8, ptr %"args::Any[]", i32 8
  %2 = load ptr, ptr %1, align 8, !tbaa !28, !invariant.load !19, !alias.scope !51, !noalias !54, !nonnull !19, !dereferenceable !59, !align !59
  %.unbox = load i8, ptr %0, align 1, !tbaa !60, !alias.scope !64, !noalias !65
  %.unbox1 = load i8, ptr %2, align 1, !tbaa !60, !alias.scope !64, !noalias !65
  %3 = call swiftcc i8 @julia_r1_gcd_225(ptr nonnull swiftself "gcstack" %tls_pgcstack, i8 signext %.unbox, i8 signext %.unbox1)
  %4 = zext i8 %3 to i32
  %5 = getelementptr inbounds nuw [256 x ptr], ptr @jl_boxed_int8_cache, i32 0, i32 %4
  %6 = load ptr, ptr %5, align 8, !tbaa !28, !invariant.load !19, !alias.scope !51, !noalias !54, !nonnull !19, !dereferenceable !59, !align !59
  ret ptr %6
}

attributes #0 = { norecurse nounwind "frame-pointer"="all" "julia.fsig"="r1_gcd(Int8, Int8)" "probe-stack"="inline-asm" }
attributes #1 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #2 = { noinline optnone "frame-pointer"="all" "probe-stack"="inline-asm" }

!llvm.dbg.cu = !{!0}
!llvm.module.flags = !{!2, !3, !4, !5, !6, !7}
!llvm.errno.tbaa = !{!8}

!0 = distinct !DICompileUnit(language: DW_LANG_Julia, file: !1, producer: "julia", isOptimized: true, runtimeVersion: 0, emissionKind: NoDebug, nameTableKind: GNU)
!1 = !DIFile(filename: "julia", directory: ".")
!2 = !{i32 2, !"Dwarf Version", i32 4}
!3 = !{i32 2, !"Debug Info Version", i32 3}
!4 = !{i32 8, !"PIC Level", i32 0}
!5 = !{i32 7, !"PIE Level", i32 2}
!6 = !{i32 1, !"ThinLTO", i32 0}
!7 = !{i32 1, !"EnableSplitLTOUnit", i32 1}
!8 = !{!9, !10, i64 0}
!9 = !{!"__libc_errno", !10, i64 0}
!10 = !{!"int", !11, i64 0}
!11 = !{!"omnipotent char", !12, i64 0}
!12 = !{!"Simple C/C++ TBAA"}
!13 = distinct !DISubprogram(name: "r1_gcd", linkageName: "julia_r1_gcd_225", scope: null, file: !14, line: 5, type: !15, scopeLine: 5, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0, retainedNodes: !20)
!14 = !DIFile(filename: "/home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-1/fixtures/julia/scalars.jl", directory: ".")
!15 = !DISubroutineType(types: !16)
!16 = !{!17, !18, !17, !17}
!17 = !DIBasicType(name: "Int8", size: 8, encoding: DW_ATE_unsigned)
!18 = distinct !DICompositeType(tag: DW_TAG_structure_type, name: "#r1_gcd", align: 8, elements: !19, runtimeLang: DW_LANG_Julia, identifier: "123207195366672")
!19 = !{}
!20 = !{!21, !22, !23}
!21 = !DILocalVariable(name: "#self#", arg: 1, scope: !13, file: !14, line: 5, type: !18)
!22 = !DILocalVariable(name: "a", arg: 2, scope: !13, file: !14, line: 5, type: !17)
!23 = !DILocalVariable(name: "b", arg: 3, scope: !13, file: !14, line: 5, type: !17)
!24 = !{!25, !25, i64 0}
!25 = !{!"jtbaa_gcframe", !26, i64 0}
!26 = !{!"jtbaa", !27, i64 0}
!27 = !{!"jtbaa"}
!28 = !{!29, !29, i64 0}
!29 = !{!"jtbaa_const", !26, i64 0}
!30 = !DILocation(line: 5, scope: !13)
!31 = !DILocation(line: 182, scope: !32, inlinedAt: !35)
!32 = distinct !DISubprogram(name: "flipsign;", linkageName: "flipsign", scope: !33, file: !33, type: !34, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0)
!33 = !DIFile(filename: "int.jl", directory: ".")
!34 = !DISubroutineType(types: !19)
!35 = !DILocation(line: 228, scope: !36, inlinedAt: !37)
!36 = distinct !DISubprogram(name: "abs;", linkageName: "abs", scope: !33, file: !33, type: !34, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0)
!37 = !DILocation(line: 6, scope: !13)
!38 = !DILocation(line: 641, scope: !39, inlinedAt: !41)
!39 = distinct !DISubprogram(name: "==;", linkageName: "==", scope: !40, file: !40, type: !34, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0)
!40 = !DIFile(filename: "promotion.jl", directory: ".")
!41 = !DILocation(line: 491, scope: !39, inlinedAt: !42)
!42 = !DILocation(line: 320, scope: !43, inlinedAt: !45)
!43 = distinct !DISubprogram(name: "!=;", linkageName: "!=", scope: !44, file: !44, type: !34, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0)
!44 = !DIFile(filename: "operators.jl", directory: ".")
!45 = !DILocation(line: 7, scope: !13)
!46 = !DILocation(line: 342, scope: !47, inlinedAt: !48)
!47 = distinct !DISubprogram(name: "rem;", linkageName: "rem", scope: !33, file: !33, type: !34, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0)
!48 = !DILocation(line: 8, scope: !13)
!49 = !DILocation(line: 590, scope: !47, inlinedAt: !50)
!50 = !DILocation(line: 10, scope: !13)
!51 = !{!52}
!52 = !{!"jnoalias_const", !53}
!53 = !{!"jnoalias"}
!54 = !{!55, !56, !57, !58}
!55 = !{!"jnoalias_gcframe", !53}
!56 = !{!"jnoalias_stack", !53}
!57 = !{!"jnoalias_data", !53}
!58 = !{!"jnoalias_typemd", !53}
!59 = !{i64 1}
!60 = !{!61, !61, i64 0}
!61 = !{!"jtbaa_immut", !62, i64 0}
!62 = !{!"jtbaa_value", !63, i64 0}
!63 = !{!"jtbaa_data", !26, i64 0}
!64 = !{!57}
!65 = !{!55, !56, !58, !52}
