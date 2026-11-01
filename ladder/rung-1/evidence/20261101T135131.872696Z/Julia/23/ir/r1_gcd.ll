; ModuleID = 'r1_gcd'
source_filename = "r1_gcd"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@jl_boxed_int8_cache = external local_unnamed_addr constant [256 x ptr]

; Function Signature: r1_gcd(Int8, Int8)
define swiftcc range(i8 0, -127) i8 @julia_r1_gcd_225(ptr nonnull swiftself "gcstack" %pgcstack_arg, i8 signext %"a::Int8", i8 signext %"b::Int8") local_unnamed_addr #0 !dbg !4 {
top:
  %ptls_field = getelementptr inbounds nuw i8, ptr %pgcstack_arg, i64 16
  %ptls_load = load ptr, ptr %ptls_field, align 8, !tbaa !15
  %0 = getelementptr inbounds nuw i8, ptr %ptls_load, i64 16
  %safepoint = load atomic ptr, ptr %0 monotonic, align 8, !tbaa !19, !invariant.load !10
  fence syncscope("singlethread") seq_cst
  %1 = load volatile i64, ptr %safepoint, align 8
  fence syncscope("singlethread") seq_cst
    #dbg_value(i8 %"a::Int8", !13, !DIExpression(), !21)
    #dbg_value(i8 %"b::Int8", !14, !DIExpression(), !21)
  %2 = call i8 @llvm.abs.i8(i8 %"a::Int8", i1 false), !dbg !22
  %3 = icmp eq i8 %"b::Int8", 0, !dbg !29
  br i1 %3, label %L13, label %L11.preheader, !dbg !36

L11.preheader:                                    ; preds = %top
  %4 = zext i8 %2 to i16, !dbg !22
  %5 = call i8 @llvm.abs.i8(i8 %"b::Int8", i1 false), !dbg !22
  %6 = zext i8 %5 to i16, !dbg !22
  br label %after_srem, !dbg !37

L13.loopexit:                                     ; preds = %after_srem
  %7 = trunc nuw i16 %value_phi2 to i8, !dbg !40
  br label %L13, !dbg !40

L13:                                              ; preds = %L13.loopexit, %top
  %value_phi1.lcssa = phi i8 [ %2, %top ], [ %7, %L13.loopexit ]
  ret i8 %value_phi1.lcssa, !dbg !41

after_srem:                                       ; preds = %after_srem, %L11.preheader
  %value_phi13 = phi i16 [ %value_phi2, %after_srem ], [ %4, %L11.preheader ]
  %value_phi2 = phi i16 [ %8, %after_srem ], [ %6, %L11.preheader ]
  %8 = urem i16 %value_phi13, %value_phi2, !dbg !37
  %9 = icmp eq i16 %8, 0, !dbg !29
  br i1 %9, label %L13.loopexit, label %after_srem, !dbg !36
}

; Function Attrs: noinline optnone
define nonnull ptr @jfptr_r1_gcd_226(ptr %"function::Core.Function", ptr noalias nocapture noundef readonly %"args::Any[]", i32 %"nargs::UInt32") local_unnamed_addr #1 {
top:
  %thread_ptr = call ptr asm "movq %fs:0, $0", "=r"()
  %tls_ppgcstack = getelementptr inbounds i8, ptr %thread_ptr, i64 -8
  %tls_pgcstack = load ptr, ptr %tls_ppgcstack, align 8
  %0 = load ptr, ptr %"args::Any[]", align 8, !tbaa !19, !invariant.load !10, !alias.scope !42, !noalias !45, !nonnull !10, !dereferenceable !50, !align !50
  %1 = getelementptr inbounds i8, ptr %"args::Any[]", i32 8
  %2 = load ptr, ptr %1, align 8, !tbaa !19, !invariant.load !10, !alias.scope !42, !noalias !45, !nonnull !10, !dereferenceable !50, !align !50
  %.unbox = load i8, ptr %0, align 1, !tbaa !51, !alias.scope !55, !noalias !56
  %.unbox1 = load i8, ptr %2, align 1, !tbaa !51, !alias.scope !55, !noalias !56
  %3 = call swiftcc i8 @julia_r1_gcd_225(ptr nonnull swiftself "gcstack" %tls_pgcstack, i8 signext %.unbox, i8 signext %.unbox1)
  %4 = zext i8 %3 to i32
  %5 = getelementptr inbounds [256 x ptr], ptr @jl_boxed_int8_cache, i32 0, i32 %4
  %6 = load ptr, ptr %5, align 8, !tbaa !19, !invariant.load !10, !alias.scope !42, !noalias !45, !nonnull !10, !dereferenceable !50, !align !50
  ret ptr %6
}

; Function Attrs: memory(argmem: readwrite, inaccessiblemem: readwrite)
declare void @julia.safepoint(ptr) local_unnamed_addr #2

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.abs.i8(i8, i1 immarg) #3

; Function Attrs: nounwind willreturn allockind("alloc") allocsize(2) memory(argmem: read, inaccessiblemem: readwrite)
declare noalias nonnull ptr @ijl_gc_small_alloc(ptr, i32, i32, i64) #4

; Function Attrs: memory(argmem: readwrite, inaccessiblemem: readwrite)
declare void @ijl_gc_queue_root(ptr) #2

; Function Attrs: nounwind willreturn allockind("alloc") allocsize(1) memory(argmem: read, inaccessiblemem: readwrite)
declare noalias nonnull ptr @ijl_gc_big_alloc(ptr, i64, i64) #5

; Function Attrs: nounwind willreturn allockind("alloc") allocsize(1) memory(argmem: read, inaccessiblemem: readwrite)
declare noalias nonnull ptr @ijl_gc_alloc_typed(ptr, i64, i64) #5

attributes #0 = { "frame-pointer"="all" "julia.fsig"="r1_gcd(Int8, Int8)" "probe-stack"="inline-asm" }
attributes #1 = { noinline optnone "frame-pointer"="all" "probe-stack"="inline-asm" }
attributes #2 = { memory(argmem: readwrite, inaccessiblemem: readwrite) }
attributes #3 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #4 = { nounwind willreturn allockind("alloc") allocsize(2) memory(argmem: read, inaccessiblemem: readwrite) }
attributes #5 = { nounwind willreturn allockind("alloc") allocsize(1) memory(argmem: read, inaccessiblemem: readwrite) }

!llvm.module.flags = !{!0, !1}
!llvm.dbg.cu = !{!2}

!0 = !{i32 2, !"Dwarf Version", i32 4}
!1 = !{i32 2, !"Debug Info Version", i32 3}
!2 = distinct !DICompileUnit(language: DW_LANG_Julia, file: !3, producer: "julia", isOptimized: true, runtimeVersion: 0, emissionKind: NoDebug, nameTableKind: GNU)
!3 = !DIFile(filename: "julia", directory: ".")
!4 = distinct !DISubprogram(name: "r1_gcd", linkageName: "julia_r1_gcd_225", scope: null, file: !5, line: 5, type: !6, scopeLine: 5, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !2, retainedNodes: !11)
!5 = !DIFile(filename: "/home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-1/fixtures/julia/scalars.jl", directory: ".")
!6 = !DISubroutineType(types: !7)
!7 = !{!8, !9, !8, !8}
!8 = !DIBasicType(name: "Int8", size: 8, encoding: DW_ATE_unsigned)
!9 = !DICompositeType(tag: DW_TAG_structure_type, name: "#r1_gcd", align: 8, elements: !10, runtimeLang: DW_LANG_Julia, identifier: "123207195366672")
!10 = !{}
!11 = !{!12, !13, !14}
!12 = !DILocalVariable(name: "#self#", arg: 1, scope: !4, file: !5, line: 5, type: !9)
!13 = !DILocalVariable(name: "a", arg: 2, scope: !4, file: !5, line: 5, type: !8)
!14 = !DILocalVariable(name: "b", arg: 3, scope: !4, file: !5, line: 5, type: !8)
!15 = !{!16, !16, i64 0}
!16 = !{!"jtbaa_gcframe", !17, i64 0}
!17 = !{!"jtbaa", !18, i64 0}
!18 = !{!"jtbaa"}
!19 = !{!20, !20, i64 0}
!20 = !{!"jtbaa_const", !17, i64 0}
!21 = !DILocation(line: 5, scope: !4)
!22 = !DILocation(line: 182, scope: !23, inlinedAt: !26)
!23 = distinct !DISubprogram(name: "flipsign;", linkageName: "flipsign", scope: !24, file: !24, type: !25, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !2)
!24 = !DIFile(filename: "int.jl", directory: ".")
!25 = !DISubroutineType(types: !10)
!26 = !DILocation(line: 228, scope: !27, inlinedAt: !28)
!27 = distinct !DISubprogram(name: "abs;", linkageName: "abs", scope: !24, file: !24, type: !25, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !2)
!28 = !DILocation(line: 6, scope: !4)
!29 = !DILocation(line: 641, scope: !30, inlinedAt: !32)
!30 = distinct !DISubprogram(name: "==;", linkageName: "==", scope: !31, file: !31, type: !25, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !2)
!31 = !DIFile(filename: "promotion.jl", directory: ".")
!32 = !DILocation(line: 491, scope: !30, inlinedAt: !33)
!33 = !DILocation(line: 320, scope: !34, inlinedAt: !36)
!34 = distinct !DISubprogram(name: "!=;", linkageName: "!=", scope: !35, file: !35, type: !25, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !2)
!35 = !DIFile(filename: "operators.jl", directory: ".")
!36 = !DILocation(line: 7, scope: !4)
!37 = !DILocation(line: 342, scope: !38, inlinedAt: !39)
!38 = distinct !DISubprogram(name: "rem;", linkageName: "rem", scope: !24, file: !24, type: !25, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !2)
!39 = !DILocation(line: 8, scope: !4)
!40 = !DILocation(line: 590, scope: !38, inlinedAt: !41)
!41 = !DILocation(line: 10, scope: !4)
!42 = !{!43}
!43 = !{!"jnoalias_const", !44}
!44 = !{!"jnoalias"}
!45 = !{!46, !47, !48, !49}
!46 = !{!"jnoalias_gcframe", !44}
!47 = !{!"jnoalias_stack", !44}
!48 = !{!"jnoalias_data", !44}
!49 = !{!"jnoalias_typemd", !44}
!50 = !{i64 1}
!51 = !{!52, !52, i64 0}
!52 = !{!"jtbaa_immut", !53, i64 0}
!53 = !{!"jtbaa_value", !54, i64 0}
!54 = !{!"jtbaa_data", !17, i64 0}
!55 = !{!48}
!56 = !{!46, !47, !49, !43}
