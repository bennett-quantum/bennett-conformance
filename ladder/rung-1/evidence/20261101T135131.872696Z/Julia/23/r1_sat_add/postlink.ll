; ModuleID = '/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-1/20261101T135131.872696Z/Julia/23/r1_sat_add/linked.bc'
source_filename = "llvm-link"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@jl_boxed_int8_cache = external local_unnamed_addr constant [256 x ptr]

; Function Attrs: norecurse nounwind
define swiftcc i8 @julia_r1_sat_add_190(ptr nofree nonnull readonly swiftself captures(none) "gcstack" %pgcstack_arg, i8 signext %"a::Int8", i8 signext %"b::Int8") local_unnamed_addr #0 !dbg !13 {
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
  %2 = tail call i8 @llvm.sadd.sat.i8(i8 %"b::Int8", i8 %"a::Int8"), !dbg !31
  ret i8 %2, !dbg !35
}

; Function Attrs: mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.sadd.sat.i8(i8, i8) #1

; Function Attrs: noinline optnone
define nonnull ptr @jfptr_r1_sat_add_191(ptr %"function::Core.Function", ptr noalias noundef readonly captures(none) %"args::Any[]", i32 %"nargs::UInt32") local_unnamed_addr #2 {
top:
  %thread_ptr = call ptr asm "movq %fs:0, $0", "=r"()
  %tls_ppgcstack = getelementptr inbounds i8, ptr %thread_ptr, i64 -8
  %tls_pgcstack = load ptr, ptr %tls_ppgcstack, align 8
  %0 = load ptr, ptr %"args::Any[]", align 8, !tbaa !28, !invariant.load !19, !alias.scope !36, !noalias !39, !nonnull !19, !dereferenceable !44, !align !44
  %1 = getelementptr inbounds nuw i8, ptr %"args::Any[]", i32 8
  %2 = load ptr, ptr %1, align 8, !tbaa !28, !invariant.load !19, !alias.scope !36, !noalias !39, !nonnull !19, !dereferenceable !44, !align !44
  %.unbox = load i8, ptr %0, align 1, !tbaa !45, !alias.scope !49, !noalias !50
  %.unbox1 = load i8, ptr %2, align 1, !tbaa !45, !alias.scope !49, !noalias !50
  %3 = call swiftcc i8 @julia_r1_sat_add_190(ptr nonnull swiftself "gcstack" %tls_pgcstack, i8 signext %.unbox, i8 signext %.unbox1)
  %4 = zext i8 %3 to i32
  %5 = getelementptr inbounds nuw [256 x ptr], ptr @jl_boxed_int8_cache, i32 0, i32 %4
  %6 = load ptr, ptr %5, align 8, !tbaa !28, !invariant.load !19, !alias.scope !36, !noalias !39, !nonnull !19, !dereferenceable !44, !align !44
  ret ptr %6
}

attributes #0 = { norecurse nounwind "frame-pointer"="all" "julia.fsig"="r1_sat_add(Int8, Int8)" "probe-stack"="inline-asm" }
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
!13 = distinct !DISubprogram(name: "r1_sat_add", linkageName: "julia_r1_sat_add_190", scope: null, file: !14, line: 1, type: !15, scopeLine: 1, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0, retainedNodes: !20)
!14 = !DIFile(filename: "/home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-1/fixtures/julia/scalars.jl", directory: ".")
!15 = !DISubroutineType(types: !16)
!16 = !{!17, !18, !17, !17}
!17 = !DIBasicType(name: "Int8", size: 8, encoding: DW_ATE_unsigned)
!18 = distinct !DICompositeType(tag: DW_TAG_structure_type, name: "#r1_sat_add", align: 8, elements: !19, runtimeLang: DW_LANG_Julia, identifier: "123207195365392")
!19 = !{}
!20 = !{!21, !22, !23}
!21 = !DILocalVariable(name: "#self#", arg: 1, scope: !13, file: !14, line: 1, type: !18)
!22 = !DILocalVariable(name: "a", arg: 2, scope: !13, file: !14, line: 1, type: !17)
!23 = !DILocalVariable(name: "b", arg: 3, scope: !13, file: !14, line: 1, type: !17)
!24 = !{!25, !25, i64 0}
!25 = !{!"jtbaa_gcframe", !26, i64 0}
!26 = !{!"jtbaa", !27, i64 0}
!27 = !{!"jtbaa"}
!28 = !{!29, !29, i64 0}
!29 = !{!"jtbaa_const", !26, i64 0}
!30 = !DILocation(line: 1, scope: !13)
!31 = !DILocation(line: 1405, scope: !32, inlinedAt: !35)
!32 = distinct !DISubprogram(name: "clamp;", linkageName: "clamp", scope: !33, file: !33, type: !34, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0)
!33 = !DIFile(filename: "intfuncs.jl", directory: ".")
!34 = !DISubroutineType(types: !19)
!35 = !DILocation(line: 2, scope: !13)
!36 = !{!37}
!37 = !{!"jnoalias_const", !38}
!38 = !{!"jnoalias"}
!39 = !{!40, !41, !42, !43}
!40 = !{!"jnoalias_gcframe", !38}
!41 = !{!"jnoalias_stack", !38}
!42 = !{!"jnoalias_data", !38}
!43 = !{!"jnoalias_typemd", !38}
!44 = !{i64 1}
!45 = !{!46, !46, i64 0}
!46 = !{!"jtbaa_immut", !47, i64 0}
!47 = !{!"jtbaa_value", !48, i64 0}
!48 = !{!"jtbaa_data", !26, i64 0}
!49 = !{!42}
!50 = !{!40, !41, !43, !37}
