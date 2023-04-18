pub fn small_array_alignment(x: &mut [i8; 4], y: [i8; 4]) {
// CHECK: store i32 %{{.*}}, i32* %{{.*}}, align 1
// CHECK: [[VAR:%[0-9]+]] = bitcast i32* %{{.*}} to [4 x i8]*
    *x = y;
}
