pub fn small_struct_alignment(x: &mut Bytes, y: Bytes) {
// CHECK: store i32 %{{.*}}, i32* %{{.*}}, align 1
// CHECK: [[VAR:%[0-9]+]] = bitcast i32* %{{.*}} to %Bytes*
    *x = y;
}
