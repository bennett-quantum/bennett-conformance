; ModuleID = '/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-2/20261031T214926.056811Z/23/input.bc'
source_filename = "/home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-2/fixtures/hashtable_marked.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

; Function Attrs: nounwind uwtable
define dso_local i64 @ht_demo_basic(i64 noundef %0) local_unnamed_addr #0 !guid !11 {
  %2 = tail call ptr @__bennett_qalloc(i64 noundef 0, i64 noundef 1) #7
  tail call void @__bennett_qfree(ptr noundef %2) #7
  %3 = tail call noalias dereferenceable_or_null(32) ptr @malloc(i64 noundef 32) #8
  %4 = tail call noalias dereferenceable_or_null(16384) ptr @malloc(i64 noundef 16384) #8
  store ptr %4, ptr %3, align 8, !tbaa !12
  %5 = tail call dereferenceable_or_null(16384) ptr @calloc(i64 1, i64 16384)
  %6 = getelementptr inbounds nuw i8, ptr %3, i64 8
  store ptr %5, ptr %6, align 8, !tbaa !17
  %7 = getelementptr inbounds nuw i8, ptr %3, i64 16
  store i64 2048, ptr %7, align 8, !tbaa !18
  %8 = getelementptr inbounds nuw i8, ptr %3, i64 24
  store i64 0, ptr %8, align 8, !tbaa !19
  br label %9

9:                                                ; preds = %9, %1
  %10 = phi i64 [ 0, %1 ], [ %28, %9 ]
  %11 = getelementptr inbounds nuw [8 x i8], ptr %4, i64 %10
  %12 = getelementptr inbounds nuw i8, ptr %11, i64 16
  store <2 x i64> splat (i64 -9223372036854775808), ptr %11, align 8, !tbaa !20
  store <2 x i64> splat (i64 -9223372036854775808), ptr %12, align 8, !tbaa !20
  %13 = getelementptr inbounds nuw i8, ptr %11, i64 32
  %14 = getelementptr inbounds nuw i8, ptr %11, i64 48
  store <2 x i64> splat (i64 -9223372036854775808), ptr %13, align 8, !tbaa !20
  store <2 x i64> splat (i64 -9223372036854775808), ptr %14, align 8, !tbaa !20
  %15 = getelementptr inbounds nuw i8, ptr %11, i64 64
  %16 = getelementptr inbounds nuw i8, ptr %11, i64 80
  store <2 x i64> splat (i64 -9223372036854775808), ptr %15, align 8, !tbaa !20
  store <2 x i64> splat (i64 -9223372036854775808), ptr %16, align 8, !tbaa !20
  %17 = getelementptr inbounds nuw i8, ptr %11, i64 96
  %18 = getelementptr inbounds nuw i8, ptr %11, i64 112
  store <2 x i64> splat (i64 -9223372036854775808), ptr %17, align 8, !tbaa !20
  store <2 x i64> splat (i64 -9223372036854775808), ptr %18, align 8, !tbaa !20
  %19 = getelementptr inbounds nuw [8 x i8], ptr %4, i64 %10
  %20 = getelementptr inbounds nuw i8, ptr %19, i64 128
  %21 = getelementptr inbounds nuw i8, ptr %19, i64 144
  store <2 x i64> splat (i64 -9223372036854775808), ptr %20, align 8, !tbaa !20
  store <2 x i64> splat (i64 -9223372036854775808), ptr %21, align 8, !tbaa !20
  %22 = getelementptr inbounds nuw i8, ptr %19, i64 160
  %23 = getelementptr inbounds nuw i8, ptr %19, i64 176
  store <2 x i64> splat (i64 -9223372036854775808), ptr %22, align 8, !tbaa !20
  store <2 x i64> splat (i64 -9223372036854775808), ptr %23, align 8, !tbaa !20
  %24 = getelementptr inbounds nuw i8, ptr %19, i64 192
  %25 = getelementptr inbounds nuw i8, ptr %19, i64 208
  store <2 x i64> splat (i64 -9223372036854775808), ptr %24, align 8, !tbaa !20
  store <2 x i64> splat (i64 -9223372036854775808), ptr %25, align 8, !tbaa !20
  %26 = getelementptr inbounds nuw i8, ptr %19, i64 224
  %27 = getelementptr inbounds nuw i8, ptr %19, i64 240
  store <2 x i64> splat (i64 -9223372036854775808), ptr %26, align 8, !tbaa !20
  store <2 x i64> splat (i64 -9223372036854775808), ptr %27, align 8, !tbaa !20
  %28 = add nuw nsw i64 %10, 32
  %29 = icmp eq i64 %28, 2048
  br i1 %29, label %30, label %9, !llvm.loop !21

30:                                               ; preds = %9
  %31 = icmp sgt i64 %0, 0
  br i1 %31, label %.preheader14, label %89

.preheader14:                                     ; preds = %30, %.loopexit13
  %32 = phi i64 [ %86, %.loopexit13 ], [ 0, %30 ]
  %33 = phi i64 [ %87, %.loopexit13 ], [ 0, %30 ]
  %34 = mul nuw nsw i64 %33, 2654435761
  %35 = add nuw nsw i64 %34, 7
  %36 = xor i64 %33, 23130
  %37 = mul nuw nsw i64 %33, 3
  %38 = add nuw nsw i64 %36, %37
  %39 = lshr i64 %35, 30
  %40 = xor i64 %39, %35
  %41 = mul i64 %40, -4658895280553007687
  %42 = lshr i64 %41, 27
  %43 = xor i64 %42, %41
  %44 = mul i64 %43, -7723592293110705685
  %45 = lshr i64 %44, 31
  %46 = xor i64 %45, %44
  br label %47

47:                                               ; preds = %81, %.preheader14
  %48 = phi i64 [ %46, %.preheader14 ], [ %83, %81 ]
  %49 = phi i64 [ -1, %.preheader14 ], [ %82, %81 ]
  %50 = phi i64 [ 0, %.preheader14 ], [ %84, %81 ]
  %51 = and i64 %48, 2047
  %52 = getelementptr inbounds nuw [8 x i8], ptr %4, i64 %51
  %53 = load i64, ptr %52, align 8, !tbaa !20
  switch i64 %53, label %65 [
    i64 -9223372036854775808, label %54
    i64 -9223372036854775807, label %62
  ]

54:                                               ; preds = %70, %47
  %55 = phi i64 [ %49, %47 ], [ %71, %70 ]
  %56 = phi i64 [ %51, %47 ], [ %73, %70 ]
  %57 = icmp slt i64 %55, 0
  %58 = select i1 %57, i64 %56, i64 %55
  %59 = getelementptr inbounds [8 x i8], ptr %4, i64 %58
  store i64 %35, ptr %59, align 8, !tbaa !20
  %60 = getelementptr inbounds [8 x i8], ptr %5, i64 %58
  store i64 %38, ptr %60, align 8, !tbaa !20
  %61 = add nsw i64 %32, 1
  store i64 %61, ptr %8, align 8, !tbaa !19
  br label %.loopexit13

62:                                               ; preds = %47
  %63 = icmp slt i64 %49, 0
  %64 = select i1 %63, i64 %51, i64 %49
  br label %70

65:                                               ; preds = %47
  %66 = icmp eq i64 %53, %35
  br i1 %66, label %67, label %70

67:                                               ; preds = %79, %65
  %68 = phi i64 [ %51, %65 ], [ %73, %79 ]
  %69 = getelementptr inbounds nuw [8 x i8], ptr %5, i64 %68
  store i64 %38, ptr %69, align 8, !tbaa !20
  br label %.loopexit13

70:                                               ; preds = %65, %62
  %71 = phi i64 [ %49, %65 ], [ %64, %62 ]
  %72 = add i64 %48, 1
  %73 = and i64 %72, 2047
  %74 = getelementptr inbounds nuw [8 x i8], ptr %4, i64 %73
  %75 = load i64, ptr %74, align 8, !tbaa !20
  switch i64 %75, label %79 [
    i64 -9223372036854775808, label %54
    i64 -9223372036854775807, label %76
  ]

76:                                               ; preds = %70
  %77 = icmp slt i64 %71, 0
  %78 = select i1 %77, i64 %73, i64 %71
  br label %81

79:                                               ; preds = %70
  %80 = icmp eq i64 %75, %35
  br i1 %80, label %67, label %81

81:                                               ; preds = %79, %76
  %82 = phi i64 [ %71, %79 ], [ %78, %76 ]
  %83 = add nuw nsw i64 %73, 1
  %84 = add nuw nsw i64 %50, 2
  %85 = icmp eq i64 %84, 2048
  br i1 %85, label %.loopexit13, label %47, !llvm.loop !25

.loopexit13:                                      ; preds = %81, %67, %54
  %86 = phi i64 [ %32, %67 ], [ %61, %54 ], [ %32, %81 ]
  %87 = add nuw nsw i64 %33, 1
  %88 = icmp eq i64 %87, %0
  br i1 %88, label %.preheader, label %.preheader14, !llvm.loop !26

89:                                               ; preds = %30
  tail call fastcc void @ht_third_pass(ptr noundef nonnull %3, i64 noundef %0)
  br label %136

90:                                               ; preds = %.loopexit12
  tail call fastcc void @ht_third_pass(ptr noundef nonnull %3, i64 noundef %0)
  %91 = load i64, ptr %7, align 8, !tbaa !18
  %92 = add nsw i64 %91, -1
  %93 = icmp sgt i64 %91, 0
  br label %142

.preheader:                                       ; preds = %.loopexit13, %.loopexit12
  %94 = phi i64 [ %131, %.loopexit12 ], [ %86, %.loopexit13 ]
  %95 = phi i64 [ %132, %.loopexit12 ], [ 0, %.loopexit13 ]
  %96 = mul nuw nsw i64 %95, 2654435761
  %97 = add nuw nsw i64 %96, 7
  %98 = lshr i64 %97, 30
  %99 = xor i64 %98, %97
  %100 = mul i64 %99, -4658895280553007687
  %101 = lshr i64 %100, 27
  %102 = xor i64 %101, %100
  %103 = mul i64 %102, -7723592293110705685
  %104 = lshr i64 %103, 31
  %105 = xor i64 %104, %103
  br label %106

106:                                              ; preds = %127, %.preheader
  %107 = phi i64 [ %105, %.preheader ], [ %128, %127 ]
  %108 = phi i64 [ 0, %.preheader ], [ %129, %127 ]
  %109 = and i64 %107, 2047
  %110 = getelementptr inbounds nuw [8 x i8], ptr %4, i64 %109
  %111 = load i64, ptr %110, align 8, !tbaa !20
  %112 = icmp eq i64 %111, -9223372036854775808
  br i1 %112, label %.loopexit12, label %113

113:                                              ; preds = %106
  %114 = icmp eq i64 %111, %97
  br i1 %114, label %115, label %119

115:                                              ; preds = %125, %113
  %116 = phi i64 [ %109, %113 ], [ %121, %125 ]
  %117 = getelementptr inbounds nuw [8 x i8], ptr %4, i64 %116
  store i64 -9223372036854775807, ptr %117, align 8, !tbaa !20
  %118 = add nsw i64 %94, -1
  store i64 %118, ptr %8, align 8, !tbaa !19
  br label %.loopexit12

119:                                              ; preds = %113
  %120 = add i64 %107, 1
  %121 = and i64 %120, 2047
  %122 = getelementptr inbounds nuw [8 x i8], ptr %4, i64 %121
  %123 = load i64, ptr %122, align 8, !tbaa !20
  %124 = icmp eq i64 %123, -9223372036854775808
  br i1 %124, label %.loopexit12, label %125

125:                                              ; preds = %119
  %126 = icmp eq i64 %123, %97
  br i1 %126, label %115, label %127

127:                                              ; preds = %125
  %128 = add nuw nsw i64 %121, 1
  %129 = add nuw nsw i64 %108, 2
  %130 = icmp eq i64 %129, 2048
  br i1 %130, label %.loopexit12, label %106, !llvm.loop !27

.loopexit12:                                      ; preds = %127, %119, %106, %115
  %131 = phi i64 [ %118, %115 ], [ %94, %106 ], [ %94, %119 ], [ %94, %127 ]
  %132 = add nuw nsw i64 %95, 3
  %133 = icmp slt i64 %132, %0
  br i1 %133, label %.preheader, label %90, !llvm.loop !28

134:                                              ; preds = %177
  %.lcssa = phi i64 [ %179, %177 ]
  %135 = mul nsw i64 %.lcssa, 1000003
  br label %136

136:                                              ; preds = %134, %89
  %137 = phi i64 [ 0, %89 ], [ %135, %134 ]
  %138 = load i64, ptr %8, align 8, !tbaa !19
  %139 = add nsw i64 %138, %137
  %140 = load ptr, ptr %3, align 8, !tbaa !12
  tail call void @free(ptr noundef %140) #7
  %141 = load ptr, ptr %6, align 8, !tbaa !17
  tail call void @free(ptr noundef %141) #7
  tail call void @free(ptr noundef %3) #7
  ret i64 %139

142:                                              ; preds = %177, %90
  %143 = phi i64 [ 0, %90 ], [ %180, %177 ]
  %144 = phi i64 [ 0, %90 ], [ %179, %177 ]
  %145 = mul nuw nsw i64 %143, 2654435761
  %146 = add nuw nsw i64 %145, 7
  br i1 %93, label %147, label %.loopexit

147:                                              ; preds = %142
  %148 = lshr i64 %146, 30
  %149 = xor i64 %148, %146
  %150 = mul i64 %149, -4658895280553007687
  %151 = lshr i64 %150, 27
  %152 = xor i64 %151, %150
  %153 = mul i64 %152, -7723592293110705685
  %154 = lshr i64 %153, 31
  %155 = xor i64 %154, %153
  %156 = load ptr, ptr %3, align 8, !tbaa !12
  br label %157

157:                                              ; preds = %166, %147
  %158 = phi i64 [ %155, %147 ], [ %167, %166 ]
  %159 = phi i64 [ 0, %147 ], [ %168, %166 ]
  %160 = and i64 %158, %92
  %161 = getelementptr inbounds [8 x i8], ptr %156, i64 %160
  %162 = load i64, ptr %161, align 8, !tbaa !20
  %163 = icmp eq i64 %162, -9223372036854775808
  br i1 %163, label %.loopexit, label %164

164:                                              ; preds = %157
  %165 = icmp eq i64 %162, %146
  br i1 %165, label %170, label %166

166:                                              ; preds = %164
  %167 = add nsw i64 %160, 1
  %168 = add nuw nsw i64 %159, 1
  %169 = icmp eq i64 %168, %91
  br i1 %169, label %.loopexit, label %157, !llvm.loop !29

170:                                              ; preds = %164
  %.lcssa30 = phi i64 [ %160, %164 ]
  %171 = load ptr, ptr %6, align 8, !tbaa !17
  %172 = getelementptr inbounds [8 x i8], ptr %171, i64 %.lcssa30
  %173 = load i64, ptr %172, align 8, !tbaa !20
  %174 = mul nuw nsw i64 %143, 131
  %175 = xor i64 %173, %174
  br label %177

.loopexit:                                        ; preds = %166, %157, %142
  %176 = xor i64 %143, -1
  br label %177

177:                                              ; preds = %.loopexit, %170
  %178 = phi i64 [ %176, %.loopexit ], [ %175, %170 ]
  %179 = add i64 %178, %144
  %180 = add nuw nsw i64 %143, 1
  %181 = icmp eq i64 %180, %0
  br i1 %181, label %134, label %142, !llvm.loop !30
}

declare ptr @__bennett_qalloc(i64 noundef, i64 noundef) local_unnamed_addr #1

declare void @__bennett_qfree(ptr noundef) local_unnamed_addr #1

; Function Attrs: nofree norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem: none) uwtable
define internal fastcc void @ht_third_pass(ptr nofree noundef captures(none) %0, i64 noundef %1) unnamed_addr #2 !guid !31 {
  %3 = icmp sgt i64 %1, 0
  br i1 %3, label %4, label %.loopexit8

4:                                                ; preds = %2
  %5 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %6 = getelementptr inbounds nuw i8, ptr %0, i64 8
  %7 = getelementptr inbounds nuw i8, ptr %0, i64 24
  br label %10

8:                                                ; preds = %.loopexit9
  %9 = icmp eq i64 %1, 1
  br i1 %9, label %.loopexit8, label %61

10:                                               ; preds = %.loopexit9, %4
  %11 = phi i64 [ 0, %4 ], [ %59, %.loopexit9 ]
  %12 = mul nuw nsw i64 %11, 2654435761
  %13 = add nuw nsw i64 %12, 7
  %14 = xor i64 %11, 23130
  %15 = mul nuw nsw i64 %11, 3
  %16 = add nuw nsw i64 %14, %15
  %17 = or disjoint i64 %16, 1
  %18 = load i64, ptr %5, align 8, !tbaa !18
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
  %30 = load ptr, ptr %0, align 8, !tbaa !12
  br label %31

31:                                               ; preds = %54, %21
  %32 = phi i64 [ %29, %21 ], [ %56, %54 ]
  %33 = phi i64 [ -1, %21 ], [ %55, %54 ]
  %34 = phi i64 [ 0, %21 ], [ %57, %54 ]
  %35 = and i64 %32, %19
  %36 = getelementptr inbounds [8 x i8], ptr %30, i64 %35
  %37 = load i64, ptr %36, align 8, !tbaa !20
  switch i64 %37, label %49 [
    i64 -9223372036854775808, label %38
    i64 -9223372036854775807, label %46
  ]

38:                                               ; preds = %31
  %.lcssa50 = phi i64 [ %33, %31 ]
  %.lcssa47 = phi i64 [ %35, %31 ]
  %39 = icmp slt i64 %.lcssa50, 0
  %40 = select i1 %39, i64 %.lcssa47, i64 %.lcssa50
  %41 = getelementptr inbounds [8 x i8], ptr %30, i64 %40
  store i64 %13, ptr %41, align 8, !tbaa !20
  %42 = load ptr, ptr %6, align 8, !tbaa !17
  %43 = getelementptr inbounds [8 x i8], ptr %42, i64 %40
  store i64 %17, ptr %43, align 8, !tbaa !20
  %44 = load i64, ptr %7, align 8, !tbaa !19
  %45 = add nsw i64 %44, 1
  store i64 %45, ptr %7, align 8, !tbaa !19
  br label %.loopexit9

46:                                               ; preds = %31
  %47 = icmp slt i64 %33, 0
  %48 = select i1 %47, i64 %35, i64 %33
  br label %54

49:                                               ; preds = %31
  %50 = icmp eq i64 %37, %13
  br i1 %50, label %51, label %54

51:                                               ; preds = %49
  %.lcssa48 = phi i64 [ %35, %49 ]
  %52 = load ptr, ptr %6, align 8, !tbaa !17
  %53 = getelementptr inbounds [8 x i8], ptr %52, i64 %.lcssa48
  store i64 %17, ptr %53, align 8, !tbaa !20
  br label %.loopexit9

54:                                               ; preds = %49, %46
  %55 = phi i64 [ %33, %49 ], [ %48, %46 ]
  %56 = add nsw i64 %35, 1
  %57 = add nuw nsw i64 %34, 1
  %58 = icmp eq i64 %57, %18
  br i1 %58, label %.loopexit9, label %31, !llvm.loop !25

.loopexit9:                                       ; preds = %54, %51, %38, %10
  %59 = add nuw nsw i64 %11, 6
  %60 = icmp slt i64 %59, %1
  br i1 %60, label %10, label %8, !llvm.loop !32

61:                                               ; preds = %8
  %62 = load i64, ptr %5, align 8, !tbaa !18
  %63 = add nsw i64 %62, -1
  %64 = icmp sgt i64 %62, 0
  br i1 %64, label %65, label %.loopexit8

65:                                               ; preds = %61
  %66 = load ptr, ptr %0, align 8, !tbaa !12
  br label %67

67:                                               ; preds = %88, %65
  %68 = phi i64 [ -4911069518324041024, %65 ], [ %90, %88 ]
  %69 = phi i64 [ -1, %65 ], [ %89, %88 ]
  %70 = phi i64 [ 0, %65 ], [ %91, %88 ]
  %71 = and i64 %68, %63
  %72 = getelementptr inbounds [8 x i8], ptr %66, i64 %71
  %73 = load i64, ptr %72, align 8, !tbaa !20
  switch i64 %73, label %88 [
    i64 -9223372036854775808, label %74
    i64 -9223372036854775807, label %82
    i64 2654435768, label %85
  ]

74:                                               ; preds = %67
  %.lcssa44 = phi i64 [ %69, %67 ]
  %.lcssa = phi i64 [ %71, %67 ]
  %75 = icmp slt i64 %.lcssa44, 0
  %76 = select i1 %75, i64 %.lcssa, i64 %.lcssa44
  %77 = getelementptr inbounds [8 x i8], ptr %66, i64 %76
  store i64 2654435768, ptr %77, align 8, !tbaa !20
  %78 = load ptr, ptr %6, align 8, !tbaa !17
  %79 = getelementptr inbounds [8 x i8], ptr %78, i64 %76
  store i64 23135, ptr %79, align 8, !tbaa !20
  %80 = load i64, ptr %7, align 8, !tbaa !19
  %81 = add nsw i64 %80, 1
  store i64 %81, ptr %7, align 8, !tbaa !19
  br label %.loopexit8

82:                                               ; preds = %67
  %83 = icmp slt i64 %69, 0
  %84 = select i1 %83, i64 %71, i64 %69
  br label %88

85:                                               ; preds = %67
  %.lcssa42 = phi i64 [ %71, %67 ]
  %86 = load ptr, ptr %6, align 8, !tbaa !17
  %87 = getelementptr inbounds [8 x i8], ptr %86, i64 %.lcssa42
  store i64 23135, ptr %87, align 8, !tbaa !20
  br label %.loopexit8

88:                                               ; preds = %82, %67
  %89 = phi i64 [ %69, %67 ], [ %84, %82 ]
  %90 = add nsw i64 %71, 1
  %91 = add nuw nsw i64 %70, 1
  %92 = icmp eq i64 %91, %62
  br i1 %92, label %.loopexit8, label %67, !llvm.loop !25

.loopexit8:                                       ; preds = %88, %85, %74, %61, %8, %2
  %93 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %94 = load i64, ptr %93, align 8, !tbaa !18
  %95 = add nsw i64 %94, -1
  %96 = icmp sgt i64 %94, 0
  br i1 %96, label %97, label %.loopexit

97:                                               ; preds = %.loopexit8
  %98 = load ptr, ptr %0, align 8, !tbaa !12
  br label %99

99:                                               ; preds = %110, %97
  %100 = phi i64 [ 2185194620014831856, %97 ], [ %111, %110 ]
  %101 = phi i64 [ 0, %97 ], [ %112, %110 ]
  %102 = and i64 %100, %95
  %103 = getelementptr inbounds [8 x i8], ptr %98, i64 %102
  %104 = load i64, ptr %103, align 8, !tbaa !20
  switch i64 %104, label %110 [
    i64 -9223372036854775808, label %.loopexit
    i64 3, label %105
  ]

105:                                              ; preds = %99
  %.lcssa53 = phi i64 [ %102, %99 ]
  %106 = getelementptr inbounds [8 x i8], ptr %98, i64 %.lcssa53
  store i64 -9223372036854775807, ptr %106, align 8, !tbaa !20
  %107 = getelementptr inbounds nuw i8, ptr %0, i64 24
  %108 = load i64, ptr %107, align 8, !tbaa !19
  %109 = add nsw i64 %108, -1
  store i64 %109, ptr %107, align 8, !tbaa !19
  br label %.loopexit

110:                                              ; preds = %99
  %111 = add nsw i64 %102, 1
  %112 = add nuw nsw i64 %101, 1
  %113 = icmp eq i64 %112, %94
  br i1 %113, label %.loopexit, label %99, !llvm.loop !27

.loopexit:                                        ; preds = %110, %99, %105, %.loopexit8
  ret void
}

; Function Attrs: nounwind uwtable
define dso_local i64 @ht_demo_grow(i64 noundef %0) local_unnamed_addr #0 !guid !33 {
  %2 = tail call ptr @__bennett_qalloc(i64 noundef 0, i64 noundef 1) #7
  tail call void @__bennett_qfree(ptr noundef %2) #7
  %3 = tail call noalias dereferenceable_or_null(32) ptr @malloc(i64 noundef 32) #8
  %4 = tail call noalias dereferenceable_or_null(32) ptr @malloc(i64 noundef 32) #8
  store ptr %4, ptr %3, align 8, !tbaa !12
  %5 = tail call dereferenceable_or_null(32) ptr @calloc(i64 1, i64 32)
  %6 = getelementptr inbounds nuw i8, ptr %3, i64 8
  store ptr %5, ptr %6, align 8, !tbaa !17
  %7 = getelementptr inbounds nuw i8, ptr %3, i64 16
  store i64 4, ptr %7, align 8, !tbaa !18
  %8 = getelementptr inbounds nuw i8, ptr %3, i64 24
  store i64 0, ptr %8, align 8, !tbaa !19
  store <4 x i64> splat (i64 -9223372036854775808), ptr %4, align 8, !tbaa !20
  %9 = icmp sgt i64 %0, 0
  br i1 %9, label %.preheader25, label %164

10:                                               ; preds = %.loopexit22
  store i64 %161, ptr %8, align 8
  store i64 %101, ptr %7, align 8
  br i1 %109, label %.preheader, label %.loopexit20

.preheader25:                                     ; preds = %1, %.loopexit22
  %11 = phi ptr [ %99, %.loopexit22 ], [ %4, %1 ]
  %12 = phi ptr [ %160, %.loopexit22 ], [ %4, %1 ]
  %13 = phi i64 [ %101, %.loopexit22 ], [ 4, %1 ]
  %14 = phi i64 [ %161, %.loopexit22 ], [ 0, %1 ]
  %15 = phi i64 [ %162, %.loopexit22 ], [ 0, %1 ]
  %16 = mul i64 %14, 10
  %17 = add i64 %16, 10
  %18 = mul nsw i64 %13, 7
  %19 = icmp sgt i64 %17, %18
  br i1 %19, label %20, label %98

20:                                               ; preds = %.preheader25
  %21 = load ptr, ptr %6, align 8, !tbaa !17
  %22 = shl i64 %13, 1
  %23 = shl i64 %13, 4
  %24 = tail call noalias ptr @malloc(i64 noundef %23) #8
  store ptr %24, ptr %3, align 8, !tbaa !12
  %25 = tail call noalias ptr @malloc(i64 noundef %23) #8
  store ptr %25, ptr %6, align 8, !tbaa !17
  %26 = icmp sgt i64 %22, 0
  br i1 %26, label %27, label %.loopexit24

27:                                               ; preds = %20
  tail call void @llvm.memset.p0.i64(ptr align 8 %25, i8 0, i64 %23, i1 false), !tbaa !20
  br label %28

28:                                               ; preds = %28, %27
  %29 = phi i64 [ 0, %27 ], [ %32, %28 ]
  %30 = getelementptr inbounds nuw [8 x i8], ptr %24, i64 %29
  %31 = getelementptr inbounds nuw i8, ptr %30, i64 16
  store <2 x i64> splat (i64 -9223372036854775808), ptr %30, align 8, !tbaa !20
  store <2 x i64> splat (i64 -9223372036854775808), ptr %31, align 8, !tbaa !20
  %32 = add nuw i64 %29, 4
  %33 = icmp eq i64 %32, %22
  br i1 %33, label %.loopexit24, label %28, !llvm.loop !34

.loopexit24:                                      ; preds = %28, %20
  %34 = icmp sgt i64 %13, 0
  br i1 %34, label %35, label %.loopexit23

35:                                               ; preds = %.loopexit24
  %36 = add nsw i64 %22, -1
  br label %37

37:                                               ; preds = %.loopexit21, %35
  %38 = phi i64 [ %94, %.loopexit21 ], [ 0, %35 ]
  %39 = phi i64 [ %95, %.loopexit21 ], [ 0, %35 ]
  %40 = getelementptr inbounds nuw [8 x i8], ptr %12, i64 %39
  %41 = load i64, ptr %40, align 8, !tbaa !20
  %42 = icmp sgt i64 %41, -9223372036854775807
  br i1 %42, label %43, label %.loopexit21

43:                                               ; preds = %37
  %44 = getelementptr inbounds nuw [8 x i8], ptr %21, i64 %39
  %45 = load i64, ptr %44, align 8, !tbaa !20
  br i1 %26, label %46, label %.loopexit21

46:                                               ; preds = %43
  %47 = lshr i64 %41, 30
  %48 = xor i64 %47, %41
  %49 = mul i64 %48, -4658895280553007687
  %50 = lshr i64 %49, 27
  %51 = xor i64 %50, %49
  %52 = mul i64 %51, -7723592293110705685
  %53 = lshr i64 %52, 31
  %54 = xor i64 %53, %52
  br label %55

55:                                               ; preds = %89, %46
  %56 = phi i64 [ %54, %46 ], [ %91, %89 ]
  %57 = phi i64 [ -1, %46 ], [ %90, %89 ]
  %58 = phi i64 [ 0, %46 ], [ %92, %89 ]
  %59 = and i64 %56, %36
  %60 = getelementptr inbounds [8 x i8], ptr %24, i64 %59
  %61 = load i64, ptr %60, align 8, !tbaa !20
  switch i64 %61, label %73 [
    i64 -9223372036854775808, label %62
    i64 -9223372036854775807, label %70
  ]

62:                                               ; preds = %78, %55
  %63 = phi i64 [ %57, %55 ], [ %79, %78 ]
  %64 = phi i64 [ %59, %55 ], [ %81, %78 ]
  %65 = icmp slt i64 %63, 0
  %66 = select i1 %65, i64 %64, i64 %63
  %67 = getelementptr inbounds [8 x i8], ptr %24, i64 %66
  store i64 %41, ptr %67, align 8, !tbaa !20
  %68 = getelementptr inbounds [8 x i8], ptr %25, i64 %66
  store i64 %45, ptr %68, align 8, !tbaa !20
  %69 = add nsw i64 %38, 1
  br label %.loopexit21

70:                                               ; preds = %55
  %71 = icmp slt i64 %57, 0
  %72 = select i1 %71, i64 %59, i64 %57
  br label %78

73:                                               ; preds = %55
  %74 = icmp eq i64 %61, %41
  br i1 %74, label %75, label %78

75:                                               ; preds = %87, %73
  %76 = phi i64 [ %59, %73 ], [ %81, %87 ]
  %77 = getelementptr inbounds [8 x i8], ptr %25, i64 %76
  store i64 %45, ptr %77, align 8, !tbaa !20
  br label %.loopexit21

78:                                               ; preds = %73, %70
  %79 = phi i64 [ %57, %73 ], [ %72, %70 ]
  %80 = add nsw i64 %59, 1
  %81 = and i64 %80, %36
  %82 = getelementptr inbounds [8 x i8], ptr %24, i64 %81
  %83 = load i64, ptr %82, align 8, !tbaa !20
  switch i64 %83, label %87 [
    i64 -9223372036854775808, label %62
    i64 -9223372036854775807, label %84
  ]

84:                                               ; preds = %78
  %85 = icmp slt i64 %79, 0
  %86 = select i1 %85, i64 %81, i64 %79
  br label %89

87:                                               ; preds = %78
  %88 = icmp eq i64 %83, %41
  br i1 %88, label %75, label %89

89:                                               ; preds = %87, %84
  %90 = phi i64 [ %79, %87 ], [ %86, %84 ]
  %91 = add nsw i64 %81, 1
  %92 = add nuw nsw i64 %58, 2
  %93 = icmp eq i64 %92, %22
  br i1 %93, label %.loopexit21, label %55, !llvm.loop !25

.loopexit21:                                      ; preds = %89, %75, %62, %43, %37
  %94 = phi i64 [ %38, %37 ], [ %38, %75 ], [ %69, %62 ], [ %38, %43 ], [ %38, %89 ]
  %95 = add nuw nsw i64 %39, 1
  %96 = icmp eq i64 %95, %13
  br i1 %96, label %.loopexit23, label %37, !llvm.loop !35

.loopexit23:                                      ; preds = %.loopexit21, %.loopexit24
  %97 = phi i64 [ 0, %.loopexit24 ], [ %94, %.loopexit21 ]
  tail call void @free(ptr noundef %12) #7
  tail call void @free(ptr noundef %21) #7
  br label %98

98:                                               ; preds = %.loopexit23, %.preheader25
  %99 = phi ptr [ %24, %.loopexit23 ], [ %11, %.preheader25 ]
  %100 = phi ptr [ %24, %.loopexit23 ], [ %12, %.preheader25 ]
  %101 = phi i64 [ %22, %.loopexit23 ], [ %13, %.preheader25 ]
  %102 = phi i64 [ %97, %.loopexit23 ], [ %14, %.preheader25 ]
  %103 = mul nuw nsw i64 %15, 2654435761
  %104 = add nuw nsw i64 %103, 7
  %105 = xor i64 %15, 23130
  %106 = mul nuw nsw i64 %15, 3
  %107 = add nuw nsw i64 %105, %106
  %108 = add nsw i64 %101, -1
  %109 = icmp sgt i64 %101, 0
  br i1 %109, label %110, label %.loopexit22

110:                                              ; preds = %98
  %111 = lshr i64 %104, 30
  %112 = xor i64 %111, %104
  %113 = mul i64 %112, -4658895280553007687
  %114 = lshr i64 %113, 27
  %115 = xor i64 %114, %113
  %116 = mul i64 %115, -7723592293110705685
  %117 = lshr i64 %116, 31
  %118 = xor i64 %117, %116
  br label %119

119:                                              ; preds = %155, %110
  %120 = phi i64 [ %118, %110 ], [ %157, %155 ]
  %121 = phi i64 [ -1, %110 ], [ %156, %155 ]
  %122 = phi i64 [ 0, %110 ], [ %158, %155 ]
  %123 = and i64 %120, %108
  %124 = getelementptr inbounds [8 x i8], ptr %99, i64 %123
  %125 = load i64, ptr %124, align 8, !tbaa !20
  switch i64 %125, label %138 [
    i64 -9223372036854775808, label %126
    i64 -9223372036854775807, label %135
  ]

126:                                              ; preds = %144, %119
  %127 = phi i64 [ %121, %119 ], [ %145, %144 ]
  %128 = phi i64 [ %123, %119 ], [ %147, %144 ]
  %129 = icmp slt i64 %127, 0
  %130 = select i1 %129, i64 %128, i64 %127
  %131 = getelementptr inbounds [8 x i8], ptr %99, i64 %130
  store i64 %104, ptr %131, align 8, !tbaa !20
  %132 = load ptr, ptr %6, align 8, !tbaa !17
  %133 = getelementptr inbounds [8 x i8], ptr %132, i64 %130
  store i64 %107, ptr %133, align 8, !tbaa !20
  %134 = add nsw i64 %102, 1
  br label %.loopexit22

135:                                              ; preds = %119
  %136 = icmp slt i64 %121, 0
  %137 = select i1 %136, i64 %123, i64 %121
  br label %144

138:                                              ; preds = %119
  %139 = icmp eq i64 %125, %104
  br i1 %139, label %140, label %144

140:                                              ; preds = %153, %138
  %141 = phi i64 [ %123, %138 ], [ %147, %153 ]
  %142 = load ptr, ptr %6, align 8, !tbaa !17
  %143 = getelementptr inbounds [8 x i8], ptr %142, i64 %141
  store i64 %107, ptr %143, align 8, !tbaa !20
  br label %.loopexit22

144:                                              ; preds = %138, %135
  %145 = phi i64 [ %121, %138 ], [ %137, %135 ]
  %146 = add nsw i64 %123, 1
  %147 = and i64 %146, %108
  %148 = getelementptr inbounds [8 x i8], ptr %99, i64 %147
  %149 = load i64, ptr %148, align 8, !tbaa !20
  switch i64 %149, label %153 [
    i64 -9223372036854775808, label %126
    i64 -9223372036854775807, label %150
  ]

150:                                              ; preds = %144
  %151 = icmp slt i64 %145, 0
  %152 = select i1 %151, i64 %147, i64 %145
  br label %155

153:                                              ; preds = %144
  %154 = icmp eq i64 %149, %104
  br i1 %154, label %140, label %155

155:                                              ; preds = %153, %150
  %156 = phi i64 [ %145, %153 ], [ %152, %150 ]
  %157 = add nsw i64 %147, 1
  %158 = add nuw nsw i64 %122, 2
  %159 = icmp eq i64 %158, %101
  br i1 %159, label %.loopexit22, label %119, !llvm.loop !25

.loopexit22:                                      ; preds = %155, %140, %126, %98
  %160 = phi ptr [ %99, %140 ], [ %100, %98 ], [ %99, %126 ], [ %99, %155 ]
  %161 = phi i64 [ %102, %140 ], [ %102, %98 ], [ %134, %126 ], [ %102, %155 ]
  %162 = add nuw nsw i64 %15, 1
  %163 = icmp eq i64 %162, %0
  br i1 %163, label %10, label %.preheader25, !llvm.loop !36

164:                                              ; preds = %1
  tail call fastcc void @ht_third_pass(ptr noundef nonnull %3, i64 noundef %0)
  %165 = load i64, ptr %7, align 8, !tbaa !18
  br label %210

.loopexit20:                                      ; preds = %.loopexit19, %10
  tail call fastcc void @ht_third_pass(ptr noundef nonnull %3, i64 noundef %0)
  %166 = load i64, ptr %7, align 8, !tbaa !18
  %167 = add nsw i64 %166, -1
  %168 = icmp sgt i64 %166, 0
  br label %219

.preheader:                                       ; preds = %10, %.loopexit19
  %169 = phi i64 [ %206, %.loopexit19 ], [ 0, %10 ]
  %170 = mul nuw nsw i64 %169, 2654435761
  %171 = add nuw nsw i64 %170, 7
  %172 = lshr i64 %171, 30
  %173 = xor i64 %172, %171
  %174 = mul i64 %173, -4658895280553007687
  %175 = lshr i64 %174, 27
  %176 = xor i64 %175, %174
  %177 = mul i64 %176, -7723592293110705685
  %178 = lshr i64 %177, 31
  %179 = xor i64 %178, %177
  br label %180

180:                                              ; preds = %202, %.preheader
  %181 = phi i64 [ %179, %.preheader ], [ %203, %202 ]
  %182 = phi i64 [ 0, %.preheader ], [ %204, %202 ]
  %183 = and i64 %181, %108
  %184 = getelementptr inbounds [8 x i8], ptr %99, i64 %183
  %185 = load i64, ptr %184, align 8, !tbaa !20
  %186 = icmp eq i64 %185, -9223372036854775808
  br i1 %186, label %.loopexit19, label %187

187:                                              ; preds = %180
  %188 = icmp eq i64 %185, %171
  br i1 %188, label %189, label %194

189:                                              ; preds = %200, %187
  %190 = phi i64 [ %183, %187 ], [ %196, %200 ]
  %191 = getelementptr inbounds [8 x i8], ptr %99, i64 %190
  store i64 -9223372036854775807, ptr %191, align 8, !tbaa !20
  %192 = load i64, ptr %8, align 8, !tbaa !19
  %193 = add nsw i64 %192, -1
  store i64 %193, ptr %8, align 8, !tbaa !19
  br label %.loopexit19

194:                                              ; preds = %187
  %195 = add nsw i64 %183, 1
  %196 = and i64 %195, %108
  %197 = getelementptr inbounds [8 x i8], ptr %99, i64 %196
  %198 = load i64, ptr %197, align 8, !tbaa !20
  %199 = icmp eq i64 %198, -9223372036854775808
  br i1 %199, label %.loopexit19, label %200

200:                                              ; preds = %194
  %201 = icmp eq i64 %198, %171
  br i1 %201, label %189, label %202

202:                                              ; preds = %200
  %203 = add nsw i64 %196, 1
  %204 = add nuw nsw i64 %182, 2
  %205 = icmp eq i64 %204, %101
  br i1 %205, label %.loopexit19, label %180, !llvm.loop !27

.loopexit19:                                      ; preds = %202, %194, %180, %189
  %206 = add nuw nsw i64 %169, 3
  %207 = icmp slt i64 %206, %0
  br i1 %207, label %.preheader, label %.loopexit20, !llvm.loop !37

208:                                              ; preds = %254
  %.lcssa = phi i64 [ %256, %254 ]
  %209 = mul nsw i64 %.lcssa, 1000003
  br label %210

210:                                              ; preds = %208, %164
  %211 = phi i64 [ %165, %164 ], [ %166, %208 ]
  %212 = phi i64 [ 0, %164 ], [ %209, %208 ]
  %213 = load i64, ptr %8, align 8, !tbaa !19
  %214 = mul nsw i64 %213, 7
  %215 = add nsw i64 %214, %212
  %216 = add nsw i64 %215, %211
  %217 = load ptr, ptr %3, align 8, !tbaa !12
  tail call void @free(ptr noundef %217) #7
  %218 = load ptr, ptr %6, align 8, !tbaa !17
  tail call void @free(ptr noundef %218) #7
  tail call void @free(ptr noundef %3) #7
  ret i64 %216

219:                                              ; preds = %254, %.loopexit20
  %220 = phi i64 [ 0, %.loopexit20 ], [ %257, %254 ]
  %221 = phi i64 [ 0, %.loopexit20 ], [ %256, %254 ]
  %222 = mul nuw nsw i64 %220, 2654435761
  %223 = add nuw nsw i64 %222, 7
  br i1 %168, label %224, label %.loopexit

224:                                              ; preds = %219
  %225 = lshr i64 %223, 30
  %226 = xor i64 %225, %223
  %227 = mul i64 %226, -4658895280553007687
  %228 = lshr i64 %227, 27
  %229 = xor i64 %228, %227
  %230 = mul i64 %229, -7723592293110705685
  %231 = lshr i64 %230, 31
  %232 = xor i64 %231, %230
  %233 = load ptr, ptr %3, align 8, !tbaa !12
  br label %234

234:                                              ; preds = %243, %224
  %235 = phi i64 [ %232, %224 ], [ %244, %243 ]
  %236 = phi i64 [ 0, %224 ], [ %245, %243 ]
  %237 = and i64 %235, %167
  %238 = getelementptr inbounds [8 x i8], ptr %233, i64 %237
  %239 = load i64, ptr %238, align 8, !tbaa !20
  %240 = icmp eq i64 %239, -9223372036854775808
  br i1 %240, label %.loopexit, label %241

241:                                              ; preds = %234
  %242 = icmp eq i64 %239, %223
  br i1 %242, label %247, label %243

243:                                              ; preds = %241
  %244 = add nsw i64 %237, 1
  %245 = add nuw nsw i64 %236, 1
  %246 = icmp eq i64 %245, %166
  br i1 %246, label %.loopexit, label %234, !llvm.loop !29

247:                                              ; preds = %241
  %.lcssa56 = phi i64 [ %237, %241 ]
  %248 = load ptr, ptr %6, align 8, !tbaa !17
  %249 = getelementptr inbounds [8 x i8], ptr %248, i64 %.lcssa56
  %250 = load i64, ptr %249, align 8, !tbaa !20
  %251 = mul nuw nsw i64 %220, 131
  %252 = xor i64 %250, %251
  br label %254

.loopexit:                                        ; preds = %243, %234, %219
  %253 = xor i64 %220, -1
  br label %254

254:                                              ; preds = %.loopexit, %247
  %255 = phi i64 [ %253, %.loopexit ], [ %252, %247 ]
  %256 = add i64 %255, %221
  %257 = add nuw nsw i64 %220, 1
  %258 = icmp eq i64 %257, %0
  br i1 %258, label %208, label %219, !llvm.loop !38
}

; Function Attrs: mustprogress nofree nounwind willreturn allockind("alloc,uninitialized") allocsize(0) memory(inaccessiblemem: readwrite, errnomem: write)
declare noalias noundef ptr @malloc(i64 noundef) local_unnamed_addr #3

; Function Attrs: mustprogress nounwind willreturn allockind("free") memory(argmem: readwrite, inaccessiblemem: readwrite)
declare void @free(ptr allocptr noundef captures(none)) local_unnamed_addr #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr writeonly captures(none), i8, i64, i1 immarg) #5

; Function Attrs: mustprogress nofree nounwind willreturn allockind("alloc,zeroed") allocsize(0,1) memory(inaccessiblemem: readwrite, errnomem: write)
declare noalias noundef ptr @calloc(i64 noundef, i64 noundef) local_unnamed_addr #6

attributes #0 = { nounwind uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nofree norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem: none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { mustprogress nofree nounwind willreturn allockind("alloc,uninitialized") allocsize(0) memory(inaccessiblemem: readwrite, errnomem: write) "alloc-family"="malloc" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { mustprogress nounwind willreturn allockind("free") memory(argmem: readwrite, inaccessiblemem: readwrite) "alloc-family"="malloc" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #5 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: write) }
attributes #6 = { mustprogress nofree nounwind willreturn allockind("alloc,zeroed") allocsize(0,1) memory(inaccessiblemem: readwrite, errnomem: write) "alloc-family"="malloc" }
attributes #7 = { nounwind }
attributes #8 = { nounwind allocsize(0) }

!llvm.module.flags = !{!0, !1, !2, !3, !4}
!llvm.ident = !{!5}
!llvm.errno.tbaa = !{!6}

!0 = !{i32 8, !"PIC Level", i32 2}
!1 = !{i32 7, !"PIE Level", i32 2}
!2 = !{i32 7, !"uwtable", i32 2}
!3 = !{i32 1, !"ThinLTO", i32 0}
!4 = !{i32 1, !"EnableSplitLTOUnit", i32 1}
!5 = !{!"Ubuntu clang version 23.1.5 (++20261028083854+a738f2737189-1~exp1~20261028203912.92)"}
!6 = !{!7, !8, i64 0}
!7 = !{!"__libc_errno", !8, i64 0}
!8 = !{!"int", !9, i64 0}
!9 = !{!"omnipotent char", !10, i64 0}
!10 = !{!"Simple C/C++ TBAA"}
!11 = !{i64 -1436915897849251014}
!12 = !{!13, !14, i64 0}
!13 = !{!"", !14, i64 0, !14, i64 8, !16, i64 16, !16, i64 24}
!14 = !{!"p1 long", !15, i64 0}
!15 = !{!"any pointer", !9, i64 0}
!16 = !{!"long", !9, i64 0}
!17 = !{!13, !14, i64 8}
!18 = !{!13, !16, i64 16}
!19 = !{!13, !16, i64 24}
!20 = !{!16, !16, i64 0}
!21 = distinct !{!21, !22, !23, !24}
!22 = !{!"llvm.loop.mustprogress"}
!23 = !{!"llvm.loop.isvectorized", i32 1}
!24 = !{!"llvm.loop.unroll.runtime.disable"}
!25 = distinct !{!25, !22}
!26 = distinct !{!26, !22}
!27 = distinct !{!27, !22}
!28 = distinct !{!28, !22}
!29 = distinct !{!29, !22}
!30 = distinct !{!30, !22}
!31 = !{i64 1345631659135498393}
!32 = distinct !{!32, !22}
!33 = !{i64 -4426198459872790070}
!34 = distinct !{!34, !22, !23, !24}
!35 = distinct !{!35, !22}
!36 = distinct !{!36, !22}
!37 = distinct !{!37, !22}
!38 = distinct !{!38, !22}
