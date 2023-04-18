pub fn small_struct_alignment(x: &mut Bytes, y: Bytes) {
// CHECK: [[VAR:%[0-9]+]] = bitcast %Bytes* %y to i32*
// CHECK: store i32 %{{.*}}, i32* [[VAR]], align 1
    *x = y;
}
