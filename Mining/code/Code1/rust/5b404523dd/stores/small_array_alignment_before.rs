pub fn small_array_alignment(x: &mut [i8; 4], y: [i8; 4]) {
// CHECK: [[VAR:%[0-9]+]] = bitcast [4 x i8]* %y to i32*
// CHECK: store i32 %{{.*}}, i32* [[VAR]], align 1
    *x = y;
}
