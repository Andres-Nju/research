pub fn add(x: f32, y: f32) -> f32 {
// CHECK: fadd float
// CHECK-NOT: fast
    x + y
}
