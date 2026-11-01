; ModuleID = 'r1_lookup'
source_filename = "r1_lookup"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@jl_boxed_int8_cache = external local_unnamed_addr constant [256 x ptr]
@"_j_const#1" = private unnamed_addr constant [8 x i8] c"\03\F9\0B\00\7F\80*\FF", align 1

; Function Signature: r1_lookup(Int8)
define swiftcc i8 @julia_r1_lookup_284(ptr nonnull swiftself "gcstack" %pgcstack_arg, i8 signext %"x::Int8") local_unnamed_addr #0 !dbg !4 {
pass:
  %ptls_field = getelementptr inbounds nuw i8, ptr %pgcstack_arg, i64 16
  %ptls_load = load ptr, ptr %ptls_field, align 8, !tbaa !14
  %0 = getelementptr inbounds nuw i8, ptr %ptls_load, i64 16
  %safepoint = load atomic ptr, ptr %0 monotonic, align 8, !tbaa !18, !invariant.load !10
  fence syncscope("singlethread") seq_cst
  %1 = load volatile i64, ptr %safepoint, align 8
  fence syncscope("singlethread") seq_cst
    #dbg_value(i8 %"x::Int8", !13, !DIExpression(), !20)
  %2 = and i8 %"x::Int8", 7, !dbg !21
  %3 = zext nneg i8 %2 to i64, !dbg !21
  %4 = getelementptr inbounds nuw i8, ptr @"_j_const#1", i64 %3, !dbg !27
  %.unbox = load i8, ptr %4, align 1, !dbg !26, !tbaa !30, !alias.scope !34, !noalias !37
  ret i8 %.unbox, !dbg !26
}

; Function Attrs: noinline optnone
define nonnull ptr @jfptr_r1_lookup_285(ptr %"function::Core.Function", ptr noalias nocapture noundef readonly %"args::Any[]", i32 %"nargs::UInt32") local_unnamed_addr #1 {
top:
  %thread_ptr = call ptr asm "movq %fs:0, $0", "=r"()
  %tls_ppgcstack = getelementptr inbounds i8, ptr %thread_ptr, i64 -8
  %tls_pgcstack = load ptr, ptr %tls_ppgcstack, align 8
  %0 = load ptr, ptr %"args::Any[]", align 8, !tbaa !18, !invariant.load !10, !alias.scope !42, !noalias !43, !nonnull !10, !dereferenceable !44, !align !44
  %.unbox = load i8, ptr %0, align 1, !tbaa !30, !alias.scope !34, !noalias !37
  %1 = call swiftcc i8 @julia_r1_lookup_284(ptr nonnull swiftself "gcstack" %tls_pgcstack, i8 signext %.unbox)
  %2 = zext i8 %1 to i32
  %3 = getelementptr inbounds [256 x ptr], ptr @jl_boxed_int8_cache, i32 0, i32 %2
  %4 = load ptr, ptr %3, align 8, !tbaa !18, !invariant.load !10, !alias.scope !42, !noalias !43, !nonnull !10, !dereferenceable !44, !align !44
  ret ptr %4
}

; Function Attrs: memory(argmem: readwrite, inaccessiblemem: readwrite)
declare void @julia.safepoint(ptr) local_unnamed_addr #2

; Function Attrs: nounwind willreturn allockind("alloc") allocsize(2) memory(argmem: read, inaccessiblemem: readwrite)
declare noalias nonnull ptr @ijl_gc_small_alloc(ptr, i32, i32, i64) #3

; Function Attrs: memory(argmem: readwrite, inaccessiblemem: readwrite)
declare void @ijl_gc_queue_root(ptr) #2

; Function Attrs: nounwind willreturn allockind("alloc") allocsize(1) memory(argmem: read, inaccessiblemem: readwrite)
declare noalias nonnull ptr @ijl_gc_big_alloc(ptr, i64, i64) #4

; Function Attrs: nounwind willreturn allockind("alloc") allocsize(1) memory(argmem: read, inaccessiblemem: readwrite)
declare noalias nonnull ptr @ijl_gc_alloc_typed(ptr, i64, i64) #4

attributes #0 = { "frame-pointer"="all" "julia.fsig"="r1_lookup(Int8)" "probe-stack"="inline-asm" }
attributes #1 = { noinline optnone "frame-pointer"="all" "probe-stack"="inline-asm" }
attributes #2 = { memory(argmem: readwrite, inaccessiblemem: readwrite) }
attributes #3 = { nounwind willreturn allockind("alloc") allocsize(2) memory(argmem: read, inaccessiblemem: readwrite) }
attributes #4 = { nounwind willreturn allockind("alloc") allocsize(1) memory(argmem: read, inaccessiblemem: readwrite) }

!llvm.module.flags = !{!0, !1}
!llvm.dbg.cu = !{!2}

!0 = !{i32 2, !"Dwarf Version", i32 4}
!1 = !{i32 2, !"Debug Info Version", i32 3}
!2 = distinct !DICompileUnit(language: DW_LANG_Julia, file: !3, producer: "julia", isOptimized: true, runtimeVersion: 0, emissionKind: NoDebug, nameTableKind: GNU)
!3 = !DIFile(filename: "julia", directory: ".")
!4 = distinct !DISubprogram(name: "r1_lookup", linkageName: "julia_r1_lookup_284", scope: null, file: !5, line: 18, type: !6, scopeLine: 18, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !2, retainedNodes: !11)
!5 = !DIFile(filename: "/home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-1/fixtures/julia/scalars.jl", directory: ".")
!6 = !DISubroutineType(types: !7)
!7 = !{!8, !9, !8}
!8 = !DIBasicType(name: "Int8", size: 8, encoding: DW_ATE_unsigned)
!9 = !DICompositeType(tag: DW_TAG_structure_type, name: "#r1_lookup", align: 8, elements: !10, runtimeLang: DW_LANG_Julia, identifier: "123207195368080")
!10 = !{}
!11 = !{!12, !13}
!12 = !DILocalVariable(name: "#self#", arg: 1, scope: !4, file: !5, line: 18, type: !9)
!13 = !DILocalVariable(name: "x", arg: 2, scope: !4, file: !5, line: 18, type: !8)
!14 = !{!15, !15, i64 0}
!15 = !{!"jtbaa_gcframe", !16, i64 0}
!16 = !{!"jtbaa", !17, i64 0}
!17 = !{!"jtbaa"}
!18 = !{!19, !19, i64 0}
!19 = !{!"jtbaa_const", !16, i64 0}
!20 = !DILocation(line: 18, scope: !4)
!21 = !DILocation(line: 393, scope: !22, inlinedAt: !25)
!22 = distinct !DISubprogram(name: "&;", linkageName: "&", scope: !23, file: !23, type: !24, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !2)
!23 = !DIFile(filename: "int.jl", directory: ".")
!24 = !DISubroutineType(types: !10)
!25 = !DILocation(line: 1058, scope: !22, inlinedAt: !26)
!26 = !DILocation(line: 20, scope: !4)
!27 = !DILocation(line: 33, scope: !28, inlinedAt: !26)
!28 = distinct !DISubprogram(name: "getindex;", linkageName: "getindex", scope: !29, file: !29, type: !24, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !2)
!29 = !DIFile(filename: "tuple.jl", directory: ".")
!30 = !{!31, !31, i64 0}
!31 = !{!"jtbaa_immut", !32, i64 0}
!32 = !{!"jtbaa_value", !33, i64 0}
!33 = !{!"jtbaa_data", !16, i64 0}
!34 = !{!35}
!35 = !{!"jnoalias_data", !36}
!36 = !{!"jnoalias"}
!37 = !{!38, !39, !40, !41}
!38 = !{!"jnoalias_gcframe", !36}
!39 = !{!"jnoalias_stack", !36}
!40 = !{!"jnoalias_typemd", !36}
!41 = !{!"jnoalias_const", !36}
!42 = !{!41}
!43 = !{!38, !39, !35, !40}
!44 = !{i64 1}
