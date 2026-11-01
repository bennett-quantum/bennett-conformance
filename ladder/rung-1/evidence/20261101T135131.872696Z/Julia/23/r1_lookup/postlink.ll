; ModuleID = '/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-1/20261101T135131.872696Z/Julia/23/r1_lookup/linked.bc'
source_filename = "llvm-link"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@jl_boxed_int8_cache = external local_unnamed_addr constant [256 x ptr]
@"_j_const#1" = private unnamed_addr constant [8 x i8] c"\03\F9\0B\00\7F\80*\FF", align 1

; Function Attrs: norecurse nounwind
define swiftcc i8 @julia_r1_lookup_284(ptr nofree nonnull readonly swiftself captures(none) "gcstack" %pgcstack_arg, i8 signext %"x::Int8") local_unnamed_addr #0 !dbg !13 {
pass:
  %ptls_field = getelementptr inbounds nuw i8, ptr %pgcstack_arg, i64 16
  %ptls_load = load ptr, ptr %ptls_field, align 8, !tbaa !23
  %0 = getelementptr inbounds nuw i8, ptr %ptls_load, i64 16
  %safepoint = load atomic ptr, ptr %0 monotonic, align 8, !tbaa !27, !invariant.load !19
  fence syncscope("singlethread") seq_cst
  %1 = load volatile i64, ptr %safepoint, align 8
  fence syncscope("singlethread") seq_cst
    #dbg_value(i8 %"x::Int8", !22, !DIExpression(), !29)
  %2 = and i8 %"x::Int8", 7, !dbg !30
  %3 = zext nneg i8 %2 to i64, !dbg !30
  %4 = getelementptr inbounds nuw i8, ptr @"_j_const#1", i64 %3, !dbg !36
  %.unbox = load i8, ptr %4, align 1, !dbg !35, !tbaa !39, !alias.scope !43, !noalias !46
  ret i8 %.unbox, !dbg !35
}

; Function Attrs: noinline optnone
define nonnull ptr @jfptr_r1_lookup_285(ptr %"function::Core.Function", ptr noalias noundef readonly captures(none) %"args::Any[]", i32 %"nargs::UInt32") local_unnamed_addr #1 {
top:
  %thread_ptr = call ptr asm "movq %fs:0, $0", "=r"()
  %tls_ppgcstack = getelementptr inbounds i8, ptr %thread_ptr, i64 -8
  %tls_pgcstack = load ptr, ptr %tls_ppgcstack, align 8
  %0 = load ptr, ptr %"args::Any[]", align 8, !tbaa !27, !invariant.load !19, !alias.scope !51, !noalias !52, !nonnull !19, !dereferenceable !53, !align !53
  %.unbox = load i8, ptr %0, align 1, !tbaa !39, !alias.scope !43, !noalias !46
  %1 = call swiftcc i8 @julia_r1_lookup_284(ptr nonnull swiftself "gcstack" %tls_pgcstack, i8 signext %.unbox)
  %2 = zext i8 %1 to i32
  %3 = getelementptr inbounds nuw [256 x ptr], ptr @jl_boxed_int8_cache, i32 0, i32 %2
  %4 = load ptr, ptr %3, align 8, !tbaa !27, !invariant.load !19, !alias.scope !51, !noalias !52, !nonnull !19, !dereferenceable !53, !align !53
  ret ptr %4
}

attributes #0 = { norecurse nounwind "frame-pointer"="all" "julia.fsig"="r1_lookup(Int8)" "probe-stack"="inline-asm" }
attributes #1 = { noinline optnone "frame-pointer"="all" "probe-stack"="inline-asm" }

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
!13 = distinct !DISubprogram(name: "r1_lookup", linkageName: "julia_r1_lookup_284", scope: null, file: !14, line: 18, type: !15, scopeLine: 18, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0, retainedNodes: !20)
!14 = !DIFile(filename: "/home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-1/fixtures/julia/scalars.jl", directory: ".")
!15 = !DISubroutineType(types: !16)
!16 = !{!17, !18, !17}
!17 = !DIBasicType(name: "Int8", size: 8, encoding: DW_ATE_unsigned)
!18 = distinct !DICompositeType(tag: DW_TAG_structure_type, name: "#r1_lookup", align: 8, elements: !19, runtimeLang: DW_LANG_Julia, identifier: "123207195368080")
!19 = !{}
!20 = !{!21, !22}
!21 = !DILocalVariable(name: "#self#", arg: 1, scope: !13, file: !14, line: 18, type: !18)
!22 = !DILocalVariable(name: "x", arg: 2, scope: !13, file: !14, line: 18, type: !17)
!23 = !{!24, !24, i64 0}
!24 = !{!"jtbaa_gcframe", !25, i64 0}
!25 = !{!"jtbaa", !26, i64 0}
!26 = !{!"jtbaa"}
!27 = !{!28, !28, i64 0}
!28 = !{!"jtbaa_const", !25, i64 0}
!29 = !DILocation(line: 18, scope: !13)
!30 = !DILocation(line: 393, scope: !31, inlinedAt: !34)
!31 = distinct !DISubprogram(name: "&;", linkageName: "&", scope: !32, file: !32, type: !33, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0)
!32 = !DIFile(filename: "int.jl", directory: ".")
!33 = !DISubroutineType(types: !19)
!34 = !DILocation(line: 1058, scope: !31, inlinedAt: !35)
!35 = !DILocation(line: 20, scope: !13)
!36 = !DILocation(line: 33, scope: !37, inlinedAt: !35)
!37 = distinct !DISubprogram(name: "getindex;", linkageName: "getindex", scope: !38, file: !38, type: !33, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0)
!38 = !DIFile(filename: "tuple.jl", directory: ".")
!39 = !{!40, !40, i64 0}
!40 = !{!"jtbaa_immut", !41, i64 0}
!41 = !{!"jtbaa_value", !42, i64 0}
!42 = !{!"jtbaa_data", !25, i64 0}
!43 = !{!44}
!44 = !{!"jnoalias_data", !45}
!45 = !{!"jnoalias"}
!46 = !{!47, !48, !49, !50}
!47 = !{!"jnoalias_gcframe", !45}
!48 = !{!"jnoalias_stack", !45}
!49 = !{!"jnoalias_typemd", !45}
!50 = !{!"jnoalias_const", !45}
!51 = !{!50}
!52 = !{!47, !48, !44, !49}
!53 = !{i64 1}
