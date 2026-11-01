; ModuleID = '/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-1/20261101T135131.872696Z/Julia/23/r1_popcount/linked.bc'
source_filename = "llvm-link"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@jl_boxed_int8_cache = external local_unnamed_addr constant [256 x ptr]

; Function Attrs: norecurse nounwind
define swiftcc range(i8 0, 9) i8 @julia_r1_popcount_244(ptr nofree nonnull readonly swiftself captures(none) "gcstack" %pgcstack_arg, i8 signext %"x::Int8") local_unnamed_addr #0 !dbg !13 {
L12:
  %ptls_field = getelementptr inbounds nuw i8, ptr %pgcstack_arg, i64 16
  %ptls_load = load ptr, ptr %ptls_field, align 8, !tbaa !23
  %0 = getelementptr inbounds nuw i8, ptr %ptls_load, i64 16
  %safepoint = load atomic ptr, ptr %0 monotonic, align 8, !tbaa !27, !invariant.load !19
  fence syncscope("singlethread") seq_cst
  %1 = load volatile i64, ptr %safepoint, align 8
  fence syncscope("singlethread") seq_cst
    #dbg_value(i8 %"x::Int8", !22, !DIExpression(), !29)
  %2 = tail call range(i8 0, 9) i8 @llvm.ctpop.i8(i8 %"x::Int8"), !dbg !30
  ret i8 %2, !dbg !29
}

; Function Attrs: mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.ctpop.i8(i8) #1

; Function Attrs: noinline optnone
define nonnull ptr @jfptr_r1_popcount_245(ptr %"function::Core.Function", ptr noalias noundef readonly captures(none) %"args::Any[]", i32 %"nargs::UInt32") local_unnamed_addr #2 {
top:
  %thread_ptr = call ptr asm "movq %fs:0, $0", "=r"()
  %tls_ppgcstack = getelementptr inbounds i8, ptr %thread_ptr, i64 -8
  %tls_pgcstack = load ptr, ptr %tls_ppgcstack, align 8
  %0 = load ptr, ptr %"args::Any[]", align 8, !tbaa !27, !invariant.load !19, !alias.scope !34, !noalias !37, !nonnull !19, !dereferenceable !42, !align !42
  %.unbox = load i8, ptr %0, align 1, !tbaa !43, !alias.scope !47, !noalias !48
  %1 = call swiftcc i8 @julia_r1_popcount_244(ptr nonnull swiftself "gcstack" %tls_pgcstack, i8 signext %.unbox)
  %2 = zext nneg i8 %1 to i32
  %3 = getelementptr inbounds nuw [256 x ptr], ptr @jl_boxed_int8_cache, i32 0, i32 %2
  %4 = load ptr, ptr %3, align 8, !tbaa !27, !invariant.load !19, !alias.scope !34, !noalias !37, !nonnull !19, !dereferenceable !42, !align !42
  ret ptr %4
}

attributes #0 = { norecurse nounwind "frame-pointer"="all" "julia.fsig"="r1_popcount(Int8)" "probe-stack"="inline-asm" }
attributes #1 = { mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none) }
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
!13 = distinct !DISubprogram(name: "r1_popcount", linkageName: "julia_r1_popcount_244", scope: null, file: !14, line: 4, type: !15, scopeLine: 4, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0, retainedNodes: !20)
!14 = !DIFile(filename: "/home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-1/fixtures/julia/scalars.jl", directory: ".")
!15 = !DISubroutineType(types: !16)
!16 = !{!17, !18, !17}
!17 = !DIBasicType(name: "Int8", size: 8, encoding: DW_ATE_unsigned)
!18 = distinct !DICompositeType(tag: DW_TAG_structure_type, name: "#r1_popcount", align: 8, elements: !19, runtimeLang: DW_LANG_Julia, identifier: "123207195365968")
!19 = !{}
!20 = !{!21, !22}
!21 = !DILocalVariable(name: "#self#", arg: 1, scope: !13, file: !14, line: 4, type: !18)
!22 = !DILocalVariable(name: "x", arg: 2, scope: !13, file: !14, line: 4, type: !17)
!23 = !{!24, !24, i64 0}
!24 = !{!"jtbaa_gcframe", !25, i64 0}
!25 = !{!"jtbaa", !26, i64 0}
!26 = !{!"jtbaa"}
!27 = !{!28, !28, i64 0}
!28 = !{!"jtbaa_const", !25, i64 0}
!29 = !DILocation(line: 4, scope: !13)
!30 = !DILocation(line: 461, scope: !31, inlinedAt: !29)
!31 = distinct !DISubprogram(name: "count_ones;", linkageName: "count_ones", scope: !32, file: !32, type: !33, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0)
!32 = !DIFile(filename: "int.jl", directory: ".")
!33 = !DISubroutineType(types: !19)
!34 = !{!35}
!35 = !{!"jnoalias_const", !36}
!36 = !{!"jnoalias"}
!37 = !{!38, !39, !40, !41}
!38 = !{!"jnoalias_gcframe", !36}
!39 = !{!"jnoalias_stack", !36}
!40 = !{!"jnoalias_data", !36}
!41 = !{!"jnoalias_typemd", !36}
!42 = !{i64 1}
!43 = !{!44, !44, i64 0}
!44 = !{!"jtbaa_immut", !45, i64 0}
!45 = !{!"jtbaa_value", !46, i64 0}
!46 = !{!"jtbaa_data", !25, i64 0}
!47 = !{!40}
!48 = !{!38, !39, !41, !35}
