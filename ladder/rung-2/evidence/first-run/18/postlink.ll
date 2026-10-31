; ModuleID = '/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-2/20261031T214926.056811Z/18/input.bc'
source_filename = "/home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-2/fixtures/hashtable_marked.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.ht = type { ptr, ptr, i64, i64 }

; Function Attrs: nounwind uwtable
define dso_local i64 @ht_demo_basic(i64 noundef %0) local_unnamed_addr #0 {
  %2 = tail call ptr @__bennett_qalloc(i64 noundef 0, i64 noundef 1) #7
  tail call void @__bennett_qfree(ptr noundef %2) #7
  %3 = tail call noalias dereferenceable_or_null(32) ptr @malloc(i64 noundef 32) #8
  %4 = tail call noalias dereferenceable_or_null(16384) ptr @malloc(i64 noundef 16384) #8
  store ptr %4, ptr %3, align 8, !tbaa !7
  %5 = tail call dereferenceable_or_null(16384) ptr @calloc(i64 1, i64 16384)
  %6 = getelementptr inbounds %struct.ht, ptr %3, i64 0, i32 1
  store ptr %5, ptr %6, align 8, !tbaa !13
  %7 = getelementptr inbounds %struct.ht, ptr %3, i64 0, i32 2
  store i64 2048, ptr %7, align 8, !tbaa !14
  %8 = getelementptr inbounds %struct.ht, ptr %3, i64 0, i32 3
  store i64 0, ptr %8, align 8, !tbaa !15
  br label %9

9:                                                ; preds = %9, %1
  %10 = phi i64 [ 0, %1 ], [ %22, %9 ]
  %11 = getelementptr inbounds i64, ptr %4, i64 %10
  %12 = getelementptr inbounds i64, ptr %11, i64 2
  store <2 x i64> <i64 -9223372036854775808, i64 -9223372036854775808>, ptr %11, align 8, !tbaa !16
  store <2 x i64> <i64 -9223372036854775808, i64 -9223372036854775808>, ptr %12, align 8, !tbaa !16
  %13 = or disjoint i64 %10, 4
  %14 = getelementptr inbounds i64, ptr %4, i64 %13
  %15 = getelementptr inbounds i64, ptr %14, i64 2
  store <2 x i64> <i64 -9223372036854775808, i64 -9223372036854775808>, ptr %14, align 8, !tbaa !16
  store <2 x i64> <i64 -9223372036854775808, i64 -9223372036854775808>, ptr %15, align 8, !tbaa !16
  %16 = or disjoint i64 %10, 8
  %17 = getelementptr inbounds i64, ptr %4, i64 %16
  %18 = getelementptr inbounds i64, ptr %17, i64 2
  store <2 x i64> <i64 -9223372036854775808, i64 -9223372036854775808>, ptr %17, align 8, !tbaa !16
  store <2 x i64> <i64 -9223372036854775808, i64 -9223372036854775808>, ptr %18, align 8, !tbaa !16
  %19 = or disjoint i64 %10, 12
  %20 = getelementptr inbounds i64, ptr %4, i64 %19
  %21 = getelementptr inbounds i64, ptr %20, i64 2
  store <2 x i64> <i64 -9223372036854775808, i64 -9223372036854775808>, ptr %20, align 8, !tbaa !16
  store <2 x i64> <i64 -9223372036854775808, i64 -9223372036854775808>, ptr %21, align 8, !tbaa !16
  %22 = add nuw nsw i64 %10, 16
  %23 = icmp eq i64 %22, 2048
  br i1 %23, label %24, label %9, !llvm.loop !17

24:                                               ; preds = %9
  %25 = icmp sgt i64 %0, 0
  br i1 %25, label %.preheader, label %83

.preheader:                                       ; preds = %24, %.loopexit13
  %26 = phi i64 [ %80, %.loopexit13 ], [ 0, %24 ]
  %27 = phi i64 [ %81, %.loopexit13 ], [ 0, %24 ]
  %28 = mul nsw i64 %27, 2654435761
  %29 = add nuw nsw i64 %28, 7
  %30 = xor i64 %27, 23130
  %31 = mul nsw i64 %27, 3
  %32 = add nuw nsw i64 %30, %31
  %33 = lshr i64 %29, 30
  %34 = xor i64 %33, %29
  %35 = mul i64 %34, -4658895280553007687
  %36 = lshr i64 %35, 27
  %37 = xor i64 %36, %35
  %38 = mul i64 %37, -7723592293110705685
  %39 = lshr i64 %38, 31
  %40 = xor i64 %39, %38
  br label %41

41:                                               ; preds = %75, %.preheader
  %42 = phi i64 [ %40, %.preheader ], [ %77, %75 ]
  %43 = phi i64 [ -1, %.preheader ], [ %76, %75 ]
  %44 = phi i64 [ 0, %.preheader ], [ %78, %75 ]
  %45 = and i64 %42, 2047
  %46 = getelementptr inbounds i64, ptr %4, i64 %45
  %47 = load i64, ptr %46, align 8, !tbaa !16
  switch i64 %47, label %59 [
    i64 -9223372036854775808, label %48
    i64 -9223372036854775807, label %56
  ]

48:                                               ; preds = %64, %41
  %49 = phi i64 [ %43, %41 ], [ %65, %64 ]
  %50 = phi i64 [ %45, %41 ], [ %67, %64 ]
  %51 = icmp slt i64 %49, 0
  %52 = select i1 %51, i64 %50, i64 %49
  %53 = getelementptr inbounds i64, ptr %4, i64 %52
  store i64 %29, ptr %53, align 8, !tbaa !16
  %54 = getelementptr inbounds i64, ptr %5, i64 %52
  store i64 %32, ptr %54, align 8, !tbaa !16
  %55 = add nsw i64 %26, 1
  store i64 %55, ptr %8, align 8, !tbaa !15
  br label %.loopexit13

56:                                               ; preds = %41
  %57 = icmp slt i64 %43, 0
  %58 = select i1 %57, i64 %45, i64 %43
  br label %64

59:                                               ; preds = %41
  %60 = icmp eq i64 %47, %29
  br i1 %60, label %61, label %64

61:                                               ; preds = %73, %59
  %62 = phi i64 [ %45, %59 ], [ %67, %73 ]
  %63 = getelementptr inbounds i64, ptr %5, i64 %62
  store i64 %32, ptr %63, align 8, !tbaa !16
  br label %.loopexit13

64:                                               ; preds = %59, %56
  %65 = phi i64 [ %43, %59 ], [ %58, %56 ]
  %66 = add i64 %42, 1
  %67 = and i64 %66, 2047
  %68 = getelementptr inbounds i64, ptr %4, i64 %67
  %69 = load i64, ptr %68, align 8, !tbaa !16
  switch i64 %69, label %73 [
    i64 -9223372036854775808, label %48
    i64 -9223372036854775807, label %70
  ]

70:                                               ; preds = %64
  %71 = icmp slt i64 %65, 0
  %72 = select i1 %71, i64 %67, i64 %65
  br label %75

73:                                               ; preds = %64
  %74 = icmp eq i64 %69, %29
  br i1 %74, label %61, label %75

75:                                               ; preds = %73, %70
  %76 = phi i64 [ %65, %73 ], [ %72, %70 ]
  %77 = add nuw nsw i64 %67, 1
  %78 = add nuw nsw i64 %44, 2
  %79 = icmp eq i64 %78, 2048
  br i1 %79, label %.loopexit13, label %41, !llvm.loop !21

.loopexit13:                                      ; preds = %75, %61, %48
  %80 = phi i64 [ %55, %48 ], [ %26, %61 ], [ %26, %75 ]
  %81 = add nuw nsw i64 %27, 1
  %82 = icmp eq i64 %81, %0
  br i1 %82, label %.preheader18, label %.preheader, !llvm.loop !22

83:                                               ; preds = %24
  tail call fastcc void @ht_third_pass(ptr noundef nonnull %3, i64 noundef %0)
  br label %130

84:                                               ; preds = %.loopexit12
  tail call fastcc void @ht_third_pass(ptr noundef nonnull %3, i64 noundef %0)
  %85 = load i64, ptr %7, align 8, !tbaa !14
  %86 = add nsw i64 %85, -1
  %87 = icmp sgt i64 %85, 0
  br label %136

.preheader18:                                     ; preds = %.loopexit13, %.loopexit12
  %88 = phi i64 [ %125, %.loopexit12 ], [ %80, %.loopexit13 ]
  %89 = phi i64 [ %126, %.loopexit12 ], [ 0, %.loopexit13 ]
  %90 = mul nsw i64 %89, 2654435761
  %91 = add nuw nsw i64 %90, 7
  %92 = lshr i64 %91, 30
  %93 = xor i64 %92, %91
  %94 = mul i64 %93, -4658895280553007687
  %95 = lshr i64 %94, 27
  %96 = xor i64 %95, %94
  %97 = mul i64 %96, -7723592293110705685
  %98 = lshr i64 %97, 31
  %99 = xor i64 %98, %97
  br label %100

100:                                              ; preds = %121, %.preheader18
  %101 = phi i64 [ %99, %.preheader18 ], [ %122, %121 ]
  %102 = phi i64 [ 0, %.preheader18 ], [ %123, %121 ]
  %103 = and i64 %101, 2047
  %104 = getelementptr inbounds i64, ptr %4, i64 %103
  %105 = load i64, ptr %104, align 8, !tbaa !16
  %106 = icmp eq i64 %105, -9223372036854775808
  br i1 %106, label %.loopexit12, label %107

107:                                              ; preds = %100
  %108 = icmp eq i64 %105, %91
  br i1 %108, label %109, label %113

109:                                              ; preds = %119, %107
  %110 = phi i64 [ %103, %107 ], [ %115, %119 ]
  %111 = getelementptr inbounds i64, ptr %4, i64 %110
  store i64 -9223372036854775807, ptr %111, align 8, !tbaa !16
  %112 = add nsw i64 %88, -1
  store i64 %112, ptr %8, align 8, !tbaa !15
  br label %.loopexit12

113:                                              ; preds = %107
  %114 = add i64 %101, 1
  %115 = and i64 %114, 2047
  %116 = getelementptr inbounds i64, ptr %4, i64 %115
  %117 = load i64, ptr %116, align 8, !tbaa !16
  %118 = icmp eq i64 %117, -9223372036854775808
  br i1 %118, label %.loopexit12, label %119

119:                                              ; preds = %113
  %120 = icmp eq i64 %117, %91
  br i1 %120, label %109, label %121

121:                                              ; preds = %119
  %122 = add nuw nsw i64 %115, 1
  %123 = add nuw nsw i64 %102, 2
  %124 = icmp eq i64 %123, 2048
  br i1 %124, label %.loopexit12, label %100, !llvm.loop !23

.loopexit12:                                      ; preds = %121, %113, %100, %109
  %125 = phi i64 [ %112, %109 ], [ %88, %100 ], [ %88, %113 ], [ %88, %121 ]
  %126 = add nuw nsw i64 %89, 3
  %127 = icmp slt i64 %126, %0
  br i1 %127, label %.preheader18, label %84, !llvm.loop !24

128:                                              ; preds = %171
  %.lcssa = phi i64 [ %173, %171 ]
  %129 = mul nsw i64 %.lcssa, 1000003
  br label %130

130:                                              ; preds = %128, %83
  %131 = phi i64 [ %129, %128 ], [ 0, %83 ]
  %132 = load i64, ptr %8, align 8, !tbaa !15
  %133 = add nsw i64 %132, %131
  %134 = load ptr, ptr %3, align 8, !tbaa !7
  tail call void @free(ptr noundef %134) #7
  %135 = load ptr, ptr %6, align 8, !tbaa !13
  tail call void @free(ptr noundef %135) #7
  tail call void @free(ptr noundef %3) #7
  ret i64 %133

136:                                              ; preds = %171, %84
  %137 = phi i64 [ 0, %84 ], [ %174, %171 ]
  %138 = phi i64 [ 0, %84 ], [ %173, %171 ]
  %139 = mul nsw i64 %137, 2654435761
  %140 = add nuw nsw i64 %139, 7
  br i1 %87, label %141, label %.loopexit

141:                                              ; preds = %136
  %142 = lshr i64 %140, 30
  %143 = xor i64 %142, %140
  %144 = mul i64 %143, -4658895280553007687
  %145 = lshr i64 %144, 27
  %146 = xor i64 %145, %144
  %147 = mul i64 %146, -7723592293110705685
  %148 = lshr i64 %147, 31
  %149 = xor i64 %148, %147
  %150 = load ptr, ptr %3, align 8, !tbaa !7
  br label %151

151:                                              ; preds = %160, %141
  %152 = phi i64 [ %149, %141 ], [ %161, %160 ]
  %153 = phi i64 [ 0, %141 ], [ %162, %160 ]
  %154 = and i64 %152, %86
  %155 = getelementptr inbounds i64, ptr %150, i64 %154
  %156 = load i64, ptr %155, align 8, !tbaa !16
  %157 = icmp eq i64 %156, -9223372036854775808
  br i1 %157, label %.loopexit, label %158

158:                                              ; preds = %151
  %159 = icmp eq i64 %156, %140
  br i1 %159, label %164, label %160

160:                                              ; preds = %158
  %161 = add nsw i64 %154, 1
  %162 = add nuw nsw i64 %153, 1
  %163 = icmp eq i64 %162, %85
  br i1 %163, label %.loopexit, label %151, !llvm.loop !25

164:                                              ; preds = %158
  %.lcssa19 = phi i64 [ %154, %158 ]
  %165 = load ptr, ptr %6, align 8, !tbaa !13
  %166 = getelementptr inbounds i64, ptr %165, i64 %.lcssa19
  %167 = load i64, ptr %166, align 8, !tbaa !16
  %168 = mul nsw i64 %137, 131
  %169 = xor i64 %167, %168
  br label %171

.loopexit:                                        ; preds = %160, %151, %136
  %170 = xor i64 %137, -1
  br label %171

171:                                              ; preds = %.loopexit, %164
  %172 = phi i64 [ %170, %.loopexit ], [ %169, %164 ]
  %173 = add i64 %172, %138
  %174 = add nuw nsw i64 %137, 1
  %175 = icmp eq i64 %174, %0
  br i1 %175, label %128, label %136, !llvm.loop !26
}

declare ptr @__bennett_qalloc(i64 noundef, i64 noundef) local_unnamed_addr #1

declare void @__bennett_qfree(ptr noundef) local_unnamed_addr #1

; Function Attrs: nofree norecurse nosync nounwind memory(readwrite, inaccessiblemem: none) uwtable
define internal fastcc void @ht_third_pass(ptr nocapture noundef %0, i64 noundef %1) unnamed_addr #2 {
  %3 = icmp sgt i64 %1, 0
  br i1 %3, label %4, label %.loopexit8

4:                                                ; preds = %2
  %5 = getelementptr inbounds %struct.ht, ptr %0, i64 0, i32 2
  %6 = getelementptr inbounds %struct.ht, ptr %0, i64 0, i32 1
  %7 = getelementptr inbounds %struct.ht, ptr %0, i64 0, i32 3
  br label %10

8:                                                ; preds = %.loopexit9
  %9 = icmp eq i64 %1, 1
  br i1 %9, label %.loopexit8, label %61

10:                                               ; preds = %.loopexit9, %4
  %11 = phi i64 [ 0, %4 ], [ %59, %.loopexit9 ]
  %12 = mul nsw i64 %11, 2654435761
  %13 = add nuw nsw i64 %12, 7
  %14 = xor i64 %11, 23130
  %15 = mul nsw i64 %11, 3
  %16 = add nuw nsw i64 %14, %15
  %17 = or disjoint i64 %16, 1
  %18 = load i64, ptr %5, align 8, !tbaa !14
  %19 = add nsw i64 %18, -1
  %20 = icmp sgt i64 %18, 0
  br i1 %20, label %21, label %.loopexit9

21:                                               ; preds = %10
  %22 = lshr i64 %13, 30
  %23 = xor i64 %22, %13
  %24 = mul i64 %23, -4658895280553007687
  %25 = lshr i64 %24, 27
  %26 = xor i64 %25, %24
  %27 = mul i64 %26, -7723592293110705685
  %28 = lshr i64 %27, 31
  %29 = xor i64 %28, %27
  %30 = load ptr, ptr %0, align 8, !tbaa !7
  br label %31

31:                                               ; preds = %54, %21
  %32 = phi i64 [ %29, %21 ], [ %56, %54 ]
  %33 = phi i64 [ -1, %21 ], [ %55, %54 ]
  %34 = phi i64 [ 0, %21 ], [ %57, %54 ]
  %35 = and i64 %32, %19
  %36 = getelementptr inbounds i64, ptr %30, i64 %35
  %37 = load i64, ptr %36, align 8, !tbaa !16
  switch i64 %37, label %49 [
    i64 -9223372036854775808, label %38
    i64 -9223372036854775807, label %46
  ]

38:                                               ; preds = %31
  %.lcssa43 = phi i64 [ %33, %31 ]
  %.lcssa40 = phi i64 [ %35, %31 ]
  %39 = icmp slt i64 %.lcssa43, 0
  %40 = select i1 %39, i64 %.lcssa40, i64 %.lcssa43
  %41 = getelementptr inbounds i64, ptr %30, i64 %40
  store i64 %13, ptr %41, align 8, !tbaa !16
  %42 = load ptr, ptr %6, align 8, !tbaa !13
  %43 = getelementptr inbounds i64, ptr %42, i64 %40
  store i64 %17, ptr %43, align 8, !tbaa !16
  %44 = load i64, ptr %7, align 8, !tbaa !15
  %45 = add nsw i64 %44, 1
  store i64 %45, ptr %7, align 8, !tbaa !15
  br label %.loopexit9

46:                                               ; preds = %31
  %47 = icmp slt i64 %33, 0
  %48 = select i1 %47, i64 %35, i64 %33
  br label %54

49:                                               ; preds = %31
  %50 = icmp eq i64 %37, %13
  br i1 %50, label %51, label %54

51:                                               ; preds = %49
  %.lcssa41 = phi i64 [ %35, %49 ]
  %52 = load ptr, ptr %6, align 8, !tbaa !13
  %53 = getelementptr inbounds i64, ptr %52, i64 %.lcssa41
  store i64 %17, ptr %53, align 8, !tbaa !16
  br label %.loopexit9

54:                                               ; preds = %49, %46
  %55 = phi i64 [ %33, %49 ], [ %48, %46 ]
  %56 = add nsw i64 %35, 1
  %57 = add nuw nsw i64 %34, 1
  %58 = icmp eq i64 %57, %18
  br i1 %58, label %.loopexit9, label %31, !llvm.loop !21

.loopexit9:                                       ; preds = %54, %51, %38, %10
  %59 = add nuw nsw i64 %11, 6
  %60 = icmp slt i64 %59, %1
  br i1 %60, label %10, label %8, !llvm.loop !27

61:                                               ; preds = %8
  %62 = load i64, ptr %5, align 8, !tbaa !14
  %63 = add nsw i64 %62, -1
  %64 = icmp sgt i64 %62, 0
  br i1 %64, label %65, label %.loopexit8

65:                                               ; preds = %61
  %66 = load ptr, ptr %0, align 8, !tbaa !7
  br label %67

67:                                               ; preds = %88, %65
  %68 = phi i64 [ -4911069518324041024, %65 ], [ %90, %88 ]
  %69 = phi i64 [ -1, %65 ], [ %89, %88 ]
  %70 = phi i64 [ 0, %65 ], [ %91, %88 ]
  %71 = and i64 %68, %63
  %72 = getelementptr inbounds i64, ptr %66, i64 %71
  %73 = load i64, ptr %72, align 8, !tbaa !16
  switch i64 %73, label %88 [
    i64 -9223372036854775808, label %74
    i64 -9223372036854775807, label %82
    i64 2654435768, label %85
  ]

74:                                               ; preds = %67
  %.lcssa37 = phi i64 [ %69, %67 ]
  %.lcssa = phi i64 [ %71, %67 ]
  %75 = icmp slt i64 %.lcssa37, 0
  %76 = select i1 %75, i64 %.lcssa, i64 %.lcssa37
  %77 = getelementptr inbounds i64, ptr %66, i64 %76
  store i64 2654435768, ptr %77, align 8, !tbaa !16
  %78 = load ptr, ptr %6, align 8, !tbaa !13
  %79 = getelementptr inbounds i64, ptr %78, i64 %76
  store i64 23135, ptr %79, align 8, !tbaa !16
  %80 = load i64, ptr %7, align 8, !tbaa !15
  %81 = add nsw i64 %80, 1
  store i64 %81, ptr %7, align 8, !tbaa !15
  br label %.loopexit8

82:                                               ; preds = %67
  %83 = icmp slt i64 %69, 0
  %84 = select i1 %83, i64 %71, i64 %69
  br label %88

85:                                               ; preds = %67
  %.lcssa35 = phi i64 [ %71, %67 ]
  %86 = load ptr, ptr %6, align 8, !tbaa !13
  %87 = getelementptr inbounds i64, ptr %86, i64 %.lcssa35
  store i64 23135, ptr %87, align 8, !tbaa !16
  br label %.loopexit8

88:                                               ; preds = %82, %67
  %89 = phi i64 [ %84, %82 ], [ %69, %67 ]
  %90 = add nsw i64 %71, 1
  %91 = add nuw nsw i64 %70, 1
  %92 = icmp eq i64 %91, %62
  br i1 %92, label %.loopexit8, label %67, !llvm.loop !21

.loopexit8:                                       ; preds = %88, %85, %74, %61, %8, %2
  %93 = getelementptr inbounds %struct.ht, ptr %0, i64 0, i32 2
  %94 = load i64, ptr %93, align 8, !tbaa !14
  %95 = add nsw i64 %94, -1
  %96 = icmp sgt i64 %94, 0
  br i1 %96, label %97, label %.loopexit

97:                                               ; preds = %.loopexit8
  %98 = load ptr, ptr %0, align 8, !tbaa !7
  br label %99

99:                                               ; preds = %110, %97
  %100 = phi i64 [ 2185194620014831856, %97 ], [ %111, %110 ]
  %101 = phi i64 [ 0, %97 ], [ %112, %110 ]
  %102 = and i64 %100, %95
  %103 = getelementptr inbounds i64, ptr %98, i64 %102
  %104 = load i64, ptr %103, align 8, !tbaa !16
  switch i64 %104, label %110 [
    i64 -9223372036854775808, label %.loopexit
    i64 3, label %105
  ]

105:                                              ; preds = %99
  %.lcssa46 = phi i64 [ %102, %99 ]
  %106 = getelementptr inbounds i64, ptr %98, i64 %.lcssa46
  store i64 -9223372036854775807, ptr %106, align 8, !tbaa !16
  %107 = getelementptr inbounds %struct.ht, ptr %0, i64 0, i32 3
  %108 = load i64, ptr %107, align 8, !tbaa !15
  %109 = add nsw i64 %108, -1
  store i64 %109, ptr %107, align 8, !tbaa !15
  br label %.loopexit

110:                                              ; preds = %99
  %111 = add nsw i64 %102, 1
  %112 = add nuw nsw i64 %101, 1
  %113 = icmp eq i64 %112, %94
  br i1 %113, label %.loopexit, label %99, !llvm.loop !23

.loopexit:                                        ; preds = %110, %99, %105, %.loopexit8
  ret void
}

; Function Attrs: nounwind uwtable
define dso_local i64 @ht_demo_grow(i64 noundef %0) local_unnamed_addr #0 {
  %2 = tail call ptr @__bennett_qalloc(i64 noundef 0, i64 noundef 1) #7
  tail call void @__bennett_qfree(ptr noundef %2) #7
  %3 = tail call noalias dereferenceable_or_null(32) ptr @malloc(i64 noundef 32) #8
  %4 = tail call noalias dereferenceable_or_null(32) ptr @malloc(i64 noundef 32) #8
  store ptr %4, ptr %3, align 8, !tbaa !7
  %5 = tail call dereferenceable_or_null(32) ptr @calloc(i64 1, i64 32)
  %6 = getelementptr inbounds %struct.ht, ptr %3, i64 0, i32 1
  store ptr %5, ptr %6, align 8, !tbaa !13
  %7 = getelementptr inbounds %struct.ht, ptr %3, i64 0, i32 2
  store i64 4, ptr %7, align 8, !tbaa !14
  %8 = getelementptr inbounds %struct.ht, ptr %3, i64 0, i32 3
  store i64 0, ptr %8, align 8, !tbaa !15
  store i64 -9223372036854775808, ptr %4, align 8, !tbaa !16
  %9 = getelementptr inbounds i64, ptr %4, i64 1
  store i64 -9223372036854775808, ptr %9, align 8, !tbaa !16
  %10 = getelementptr inbounds i64, ptr %4, i64 2
  store i64 -9223372036854775808, ptr %10, align 8, !tbaa !16
  %11 = getelementptr inbounds i64, ptr %4, i64 3
  store i64 -9223372036854775808, ptr %11, align 8, !tbaa !16
  %12 = icmp sgt i64 %0, 0
  br i1 %12, label %.preheader, label %163

13:                                               ; preds = %.loopexit20
  %.lcssa66 = phi i64 [ %160, %.loopexit20 ]
  %.lcssa65 = phi ptr [ %112, %.loopexit20 ]
  %.lcssa64 = phi i64 [ %114, %.loopexit20 ]
  %.lcssa63 = phi i64 [ %121, %.loopexit20 ]
  %.lcssa62 = phi i1 [ %122, %.loopexit20 ]
  store i64 %.lcssa66, ptr %8, align 8, !tbaa !15
  store i64 %.lcssa64, ptr %7, align 8, !tbaa !14
  br label %169

.preheader:                                       ; preds = %1, %.loopexit20
  %14 = phi ptr [ %112, %.loopexit20 ], [ %4, %1 ]
  %15 = phi ptr [ %159, %.loopexit20 ], [ %4, %1 ]
  %16 = phi i64 [ %114, %.loopexit20 ], [ 4, %1 ]
  %17 = phi i64 [ %160, %.loopexit20 ], [ 0, %1 ]
  %18 = phi i64 [ %161, %.loopexit20 ], [ 0, %1 ]
  %19 = mul i64 %17, 10
  %20 = add i64 %19, 10
  %21 = mul nsw i64 %16, 7
  %22 = icmp sgt i64 %20, %21
  br i1 %22, label %23, label %111

23:                                               ; preds = %.preheader
  %24 = load ptr, ptr %6, align 8, !tbaa !13
  %25 = shl i64 %16, 1
  %26 = shl i64 %16, 4
  %27 = tail call noalias ptr @malloc(i64 noundef %26) #8
  store ptr %27, ptr %3, align 8, !tbaa !7
  %28 = tail call noalias ptr @malloc(i64 noundef %26) #8
  store ptr %28, ptr %6, align 8, !tbaa !13
  %29 = icmp sgt i64 %25, 0
  br i1 %29, label %30, label %.loopexit22

30:                                               ; preds = %23
  tail call void @llvm.memset.p0.i64(ptr align 8 %28, i8 0, i64 %26, i1 false), !tbaa !16
  %31 = icmp ult i64 %25, 4
  br i1 %31, label %.preheader51, label %32

.preheader51:                                     ; preds = %40, %30
  %.ph = phi i64 [ %33, %40 ], [ 0, %30 ]
  br label %45

32:                                               ; preds = %30
  %33 = and i64 %25, 9223372036854775804
  br label %34

34:                                               ; preds = %34, %32
  %35 = phi i64 [ 0, %32 ], [ %38, %34 ]
  %36 = getelementptr inbounds i64, ptr %27, i64 %35
  %37 = getelementptr inbounds i64, ptr %36, i64 2
  store <2 x i64> <i64 -9223372036854775808, i64 -9223372036854775808>, ptr %36, align 8, !tbaa !16
  store <2 x i64> <i64 -9223372036854775808, i64 -9223372036854775808>, ptr %37, align 8, !tbaa !16
  %38 = add nuw i64 %35, 4
  %39 = icmp eq i64 %38, %33
  br i1 %39, label %40, label %34, !llvm.loop !28

40:                                               ; preds = %34
  %41 = icmp eq i64 %25, %33
  br i1 %41, label %.loopexit22, label %.preheader51

.loopexit22:                                      ; preds = %45, %40, %23
  %42 = icmp sgt i64 %16, 0
  br i1 %42, label %43, label %.loopexit21

43:                                               ; preds = %.loopexit22
  %44 = add nsw i64 %25, -1
  br label %50

45:                                               ; preds = %.preheader51, %45
  %46 = phi i64 [ %48, %45 ], [ %.ph, %.preheader51 ]
  %47 = getelementptr inbounds i64, ptr %27, i64 %46
  store i64 -9223372036854775808, ptr %47, align 8, !tbaa !16
  %48 = add nuw nsw i64 %46, 1
  %49 = icmp eq i64 %48, %25
  br i1 %49, label %.loopexit22, label %45, !llvm.loop !29

50:                                               ; preds = %.loopexit19, %43
  %51 = phi i64 [ %107, %.loopexit19 ], [ 0, %43 ]
  %52 = phi i64 [ %108, %.loopexit19 ], [ 0, %43 ]
  %53 = getelementptr inbounds i64, ptr %15, i64 %52
  %54 = load i64, ptr %53, align 8, !tbaa !16
  %55 = icmp sgt i64 %54, -9223372036854775807
  br i1 %55, label %56, label %.loopexit19

56:                                               ; preds = %50
  %57 = getelementptr inbounds i64, ptr %24, i64 %52
  %58 = load i64, ptr %57, align 8, !tbaa !16
  br i1 %29, label %59, label %.loopexit19

59:                                               ; preds = %56
  %60 = lshr i64 %54, 30
  %61 = xor i64 %60, %54
  %62 = mul i64 %61, -4658895280553007687
  %63 = lshr i64 %62, 27
  %64 = xor i64 %63, %62
  %65 = mul i64 %64, -7723592293110705685
  %66 = lshr i64 %65, 31
  %67 = xor i64 %66, %65
  br label %68

68:                                               ; preds = %102, %59
  %69 = phi i64 [ %67, %59 ], [ %104, %102 ]
  %70 = phi i64 [ -1, %59 ], [ %103, %102 ]
  %71 = phi i64 [ 0, %59 ], [ %105, %102 ]
  %72 = and i64 %69, %44
  %73 = getelementptr inbounds i64, ptr %27, i64 %72
  %74 = load i64, ptr %73, align 8, !tbaa !16
  switch i64 %74, label %86 [
    i64 -9223372036854775808, label %75
    i64 -9223372036854775807, label %83
  ]

75:                                               ; preds = %91, %68
  %76 = phi i64 [ %70, %68 ], [ %92, %91 ]
  %77 = phi i64 [ %72, %68 ], [ %94, %91 ]
  %78 = icmp slt i64 %76, 0
  %79 = select i1 %78, i64 %77, i64 %76
  %80 = getelementptr inbounds i64, ptr %27, i64 %79
  store i64 %54, ptr %80, align 8, !tbaa !16
  %81 = getelementptr inbounds i64, ptr %28, i64 %79
  store i64 %58, ptr %81, align 8, !tbaa !16
  %82 = add nsw i64 %51, 1
  br label %.loopexit19

83:                                               ; preds = %68
  %84 = icmp slt i64 %70, 0
  %85 = select i1 %84, i64 %72, i64 %70
  br label %91

86:                                               ; preds = %68
  %87 = icmp eq i64 %74, %54
  br i1 %87, label %88, label %91

88:                                               ; preds = %100, %86
  %89 = phi i64 [ %72, %86 ], [ %94, %100 ]
  %90 = getelementptr inbounds i64, ptr %28, i64 %89
  store i64 %58, ptr %90, align 8, !tbaa !16
  br label %.loopexit19

91:                                               ; preds = %86, %83
  %92 = phi i64 [ %70, %86 ], [ %85, %83 ]
  %93 = add nsw i64 %72, 1
  %94 = and i64 %93, %44
  %95 = getelementptr inbounds i64, ptr %27, i64 %94
  %96 = load i64, ptr %95, align 8, !tbaa !16
  switch i64 %96, label %100 [
    i64 -9223372036854775808, label %75
    i64 -9223372036854775807, label %97
  ]

97:                                               ; preds = %91
  %98 = icmp slt i64 %92, 0
  %99 = select i1 %98, i64 %94, i64 %92
  br label %102

100:                                              ; preds = %91
  %101 = icmp eq i64 %96, %54
  br i1 %101, label %88, label %102

102:                                              ; preds = %100, %97
  %103 = phi i64 [ %92, %100 ], [ %99, %97 ]
  %104 = add nsw i64 %94, 1
  %105 = add nuw nsw i64 %71, 2
  %106 = icmp eq i64 %105, %25
  br i1 %106, label %.loopexit19, label %68, !llvm.loop !21

.loopexit19:                                      ; preds = %102, %88, %75, %56, %50
  %107 = phi i64 [ %51, %88 ], [ %82, %75 ], [ %51, %56 ], [ %51, %50 ], [ %51, %102 ]
  %108 = add nuw nsw i64 %52, 1
  %109 = icmp eq i64 %108, %16
  br i1 %109, label %.loopexit21, label %50, !llvm.loop !30

.loopexit21:                                      ; preds = %.loopexit19, %.loopexit22
  %110 = phi i64 [ 0, %.loopexit22 ], [ %107, %.loopexit19 ]
  tail call void @free(ptr noundef %15) #7
  tail call void @free(ptr noundef %24) #7
  br label %111

111:                                              ; preds = %.loopexit21, %.preheader
  %112 = phi ptr [ %27, %.loopexit21 ], [ %14, %.preheader ]
  %113 = phi ptr [ %27, %.loopexit21 ], [ %15, %.preheader ]
  %114 = phi i64 [ %25, %.loopexit21 ], [ %16, %.preheader ]
  %115 = phi i64 [ %110, %.loopexit21 ], [ %17, %.preheader ]
  %116 = mul nsw i64 %18, 2654435761
  %117 = add nuw nsw i64 %116, 7
  %118 = xor i64 %18, 23130
  %119 = mul nsw i64 %18, 3
  %120 = add nuw nsw i64 %118, %119
  %121 = add nsw i64 %114, -1
  %122 = icmp sgt i64 %114, 0
  br i1 %122, label %123, label %.loopexit20

123:                                              ; preds = %111
  %124 = lshr i64 %117, 30
  %125 = xor i64 %124, %117
  %126 = mul i64 %125, -4658895280553007687
  %127 = lshr i64 %126, 27
  %128 = xor i64 %127, %126
  %129 = mul i64 %128, -7723592293110705685
  %130 = lshr i64 %129, 31
  %131 = xor i64 %130, %129
  br label %132

132:                                              ; preds = %154, %123
  %133 = phi i64 [ %131, %123 ], [ %156, %154 ]
  %134 = phi i64 [ -1, %123 ], [ %155, %154 ]
  %135 = phi i64 [ 0, %123 ], [ %157, %154 ]
  %136 = and i64 %133, %121
  %137 = getelementptr inbounds i64, ptr %112, i64 %136
  %138 = load i64, ptr %137, align 8, !tbaa !16
  switch i64 %138, label %149 [
    i64 -9223372036854775808, label %139
    i64 -9223372036854775807, label %146
  ]

139:                                              ; preds = %132
  %.lcssa59 = phi i64 [ %134, %132 ]
  %.lcssa56 = phi i64 [ %136, %132 ]
  %140 = icmp slt i64 %.lcssa59, 0
  %141 = select i1 %140, i64 %.lcssa56, i64 %.lcssa59
  %142 = getelementptr inbounds i64, ptr %112, i64 %141
  store i64 %117, ptr %142, align 8, !tbaa !16
  %143 = load ptr, ptr %6, align 8, !tbaa !13
  %144 = getelementptr inbounds i64, ptr %143, i64 %141
  store i64 %120, ptr %144, align 8, !tbaa !16
  %145 = add nsw i64 %115, 1
  br label %.loopexit20

146:                                              ; preds = %132
  %147 = icmp slt i64 %134, 0
  %148 = select i1 %147, i64 %136, i64 %134
  br label %154

149:                                              ; preds = %132
  %150 = icmp eq i64 %138, %117
  br i1 %150, label %151, label %154

151:                                              ; preds = %149
  %.lcssa57 = phi i64 [ %136, %149 ]
  %152 = load ptr, ptr %6, align 8, !tbaa !13
  %153 = getelementptr inbounds i64, ptr %152, i64 %.lcssa57
  store i64 %120, ptr %153, align 8, !tbaa !16
  br label %.loopexit20

154:                                              ; preds = %149, %146
  %155 = phi i64 [ %134, %149 ], [ %148, %146 ]
  %156 = add nsw i64 %136, 1
  %157 = add nuw nsw i64 %135, 1
  %158 = icmp eq i64 %157, %114
  br i1 %158, label %.loopexit20, label %132, !llvm.loop !21

.loopexit20:                                      ; preds = %154, %151, %139, %111
  %159 = phi ptr [ %113, %111 ], [ %112, %139 ], [ %112, %151 ], [ %112, %154 ]
  %160 = phi i64 [ %115, %111 ], [ %145, %139 ], [ %115, %151 ], [ %115, %154 ]
  %161 = add nuw nsw i64 %18, 1
  %162 = icmp eq i64 %161, %0
  br i1 %162, label %13, label %.preheader, !llvm.loop !31

163:                                              ; preds = %1
  tail call fastcc void @ht_third_pass(ptr noundef nonnull %3, i64 noundef %0)
  %164 = load i64, ptr %7, align 8, !tbaa !14
  br label %203

165:                                              ; preds = %.loopexit18
  tail call fastcc void @ht_third_pass(ptr noundef nonnull %3, i64 noundef %0)
  %166 = load i64, ptr %7, align 8, !tbaa !14
  %167 = add nsw i64 %166, -1
  %168 = icmp sgt i64 %166, 0
  br label %212

169:                                              ; preds = %.loopexit18, %13
  %170 = phi i64 [ 0, %13 ], [ %199, %.loopexit18 ]
  %171 = mul nsw i64 %170, 2654435761
  %172 = add nuw nsw i64 %171, 7
  br i1 %.lcssa62, label %173, label %.loopexit18

173:                                              ; preds = %169
  %174 = lshr i64 %172, 30
  %175 = xor i64 %174, %172
  %176 = mul i64 %175, -4658895280553007687
  %177 = lshr i64 %176, 27
  %178 = xor i64 %177, %176
  %179 = mul i64 %178, -7723592293110705685
  %180 = lshr i64 %179, 31
  %181 = xor i64 %180, %179
  br label %182

182:                                              ; preds = %195, %173
  %183 = phi i64 [ %181, %173 ], [ %196, %195 ]
  %184 = phi i64 [ 0, %173 ], [ %197, %195 ]
  %185 = and i64 %183, %.lcssa63
  %186 = getelementptr inbounds i64, ptr %.lcssa65, i64 %185
  %187 = load i64, ptr %186, align 8, !tbaa !16
  %188 = icmp eq i64 %187, -9223372036854775808
  br i1 %188, label %.loopexit18, label %189

189:                                              ; preds = %182
  %190 = icmp eq i64 %187, %172
  br i1 %190, label %191, label %195

191:                                              ; preds = %189
  %.lcssa67 = phi i64 [ %185, %189 ]
  %192 = getelementptr inbounds i64, ptr %.lcssa65, i64 %.lcssa67
  store i64 -9223372036854775807, ptr %192, align 8, !tbaa !16
  %193 = load i64, ptr %8, align 8, !tbaa !15
  %194 = add nsw i64 %193, -1
  store i64 %194, ptr %8, align 8, !tbaa !15
  br label %.loopexit18

195:                                              ; preds = %189
  %196 = add nsw i64 %185, 1
  %197 = add nuw nsw i64 %184, 1
  %198 = icmp eq i64 %197, %.lcssa64
  br i1 %198, label %.loopexit18, label %182, !llvm.loop !23

.loopexit18:                                      ; preds = %195, %182, %191, %169
  %199 = add nuw nsw i64 %170, 3
  %200 = icmp slt i64 %199, %0
  br i1 %200, label %169, label %165, !llvm.loop !32

201:                                              ; preds = %247
  %.lcssa = phi i64 [ %249, %247 ]
  %202 = mul nsw i64 %.lcssa, 1000003
  br label %203

203:                                              ; preds = %201, %163
  %204 = phi i64 [ %166, %201 ], [ %164, %163 ]
  %205 = phi i64 [ %202, %201 ], [ 0, %163 ]
  %206 = load i64, ptr %8, align 8, !tbaa !15
  %207 = mul nsw i64 %206, 7
  %208 = add nsw i64 %207, %205
  %209 = add nsw i64 %208, %204
  %210 = load ptr, ptr %3, align 8, !tbaa !7
  tail call void @free(ptr noundef %210) #7
  %211 = load ptr, ptr %6, align 8, !tbaa !13
  tail call void @free(ptr noundef %211) #7
  tail call void @free(ptr noundef %3) #7
  ret i64 %209

212:                                              ; preds = %247, %165
  %213 = phi i64 [ 0, %165 ], [ %250, %247 ]
  %214 = phi i64 [ 0, %165 ], [ %249, %247 ]
  %215 = mul nsw i64 %213, 2654435761
  %216 = add nuw nsw i64 %215, 7
  br i1 %168, label %217, label %.loopexit

217:                                              ; preds = %212
  %218 = lshr i64 %216, 30
  %219 = xor i64 %218, %216
  %220 = mul i64 %219, -4658895280553007687
  %221 = lshr i64 %220, 27
  %222 = xor i64 %221, %220
  %223 = mul i64 %222, -7723592293110705685
  %224 = lshr i64 %223, 31
  %225 = xor i64 %224, %223
  %226 = load ptr, ptr %3, align 8, !tbaa !7
  br label %227

227:                                              ; preds = %236, %217
  %228 = phi i64 [ %225, %217 ], [ %237, %236 ]
  %229 = phi i64 [ 0, %217 ], [ %238, %236 ]
  %230 = and i64 %228, %167
  %231 = getelementptr inbounds i64, ptr %226, i64 %230
  %232 = load i64, ptr %231, align 8, !tbaa !16
  %233 = icmp eq i64 %232, -9223372036854775808
  br i1 %233, label %.loopexit, label %234

234:                                              ; preds = %227
  %235 = icmp eq i64 %232, %216
  br i1 %235, label %240, label %236

236:                                              ; preds = %234
  %237 = add nsw i64 %230, 1
  %238 = add nuw nsw i64 %229, 1
  %239 = icmp eq i64 %238, %166
  br i1 %239, label %.loopexit, label %227, !llvm.loop !25

240:                                              ; preds = %234
  %.lcssa52 = phi i64 [ %230, %234 ]
  %241 = load ptr, ptr %6, align 8, !tbaa !13
  %242 = getelementptr inbounds i64, ptr %241, i64 %.lcssa52
  %243 = load i64, ptr %242, align 8, !tbaa !16
  %244 = mul nsw i64 %213, 131
  %245 = xor i64 %243, %244
  br label %247

.loopexit:                                        ; preds = %236, %227, %212
  %246 = xor i64 %213, -1
  br label %247

247:                                              ; preds = %.loopexit, %240
  %248 = phi i64 [ %246, %.loopexit ], [ %245, %240 ]
  %249 = add i64 %248, %214
  %250 = add nuw nsw i64 %213, 1
  %251 = icmp eq i64 %250, %0
  br i1 %251, label %201, label %212, !llvm.loop !33
}

; Function Attrs: mustprogress nofree nounwind willreturn allockind("alloc,uninitialized") allocsize(0) memory(inaccessiblemem: readwrite)
declare noalias noundef ptr @malloc(i64 noundef) local_unnamed_addr #3

; Function Attrs: mustprogress nounwind willreturn allockind("free") memory(argmem: readwrite, inaccessiblemem: readwrite)
declare void @free(ptr allocptr nocapture noundef) local_unnamed_addr #4

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #5

; Function Attrs: mustprogress nofree nounwind willreturn allockind("alloc,zeroed") allocsize(0,1) memory(inaccessiblemem: readwrite)
declare noalias noundef ptr @calloc(i64 noundef, i64 noundef) local_unnamed_addr #6

attributes #0 = { nounwind uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nofree norecurse nosync nounwind memory(readwrite, inaccessiblemem: none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { mustprogress nofree nounwind willreturn allockind("alloc,uninitialized") allocsize(0) memory(inaccessiblemem: readwrite) "alloc-family"="malloc" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { mustprogress nounwind willreturn allockind("free") memory(argmem: readwrite, inaccessiblemem: readwrite) "alloc-family"="malloc" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #5 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #6 = { mustprogress nofree nounwind willreturn allockind("alloc,zeroed") allocsize(0,1) memory(inaccessiblemem: readwrite) "alloc-family"="malloc" }
attributes #7 = { nounwind }
attributes #8 = { nounwind allocsize(0) }

!llvm.module.flags = !{!0, !1, !2, !3, !4, !5}
!llvm.ident = !{!6}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 8, !"PIC Level", i32 2}
!2 = !{i32 7, !"PIE Level", i32 2}
!3 = !{i32 7, !"uwtable", i32 2}
!4 = !{i32 1, !"ThinLTO", i32 0}
!5 = !{i32 1, !"EnableSplitLTOUnit", i32 1}
!6 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
!7 = !{!8, !9, i64 0}
!8 = !{!"", !9, i64 0, !9, i64 8, !12, i64 16, !12, i64 24}
!9 = !{!"any pointer", !10, i64 0}
!10 = !{!"omnipotent char", !11, i64 0}
!11 = !{!"Simple C/C++ TBAA"}
!12 = !{!"long", !10, i64 0}
!13 = !{!8, !9, i64 8}
!14 = !{!8, !12, i64 16}
!15 = !{!8, !12, i64 24}
!16 = !{!12, !12, i64 0}
!17 = distinct !{!17, !18, !19, !20}
!18 = !{!"llvm.loop.mustprogress"}
!19 = !{!"llvm.loop.isvectorized", i32 1}
!20 = !{!"llvm.loop.unroll.runtime.disable"}
!21 = distinct !{!21, !18}
!22 = distinct !{!22, !18}
!23 = distinct !{!23, !18}
!24 = distinct !{!24, !18}
!25 = distinct !{!25, !18}
!26 = distinct !{!26, !18}
!27 = distinct !{!27, !18}
!28 = distinct !{!28, !18, !19, !20}
!29 = distinct !{!29, !18, !20, !19}
!30 = distinct !{!30, !18}
!31 = distinct !{!31, !18}
!32 = distinct !{!32, !18}
!33 = distinct !{!33, !18}
