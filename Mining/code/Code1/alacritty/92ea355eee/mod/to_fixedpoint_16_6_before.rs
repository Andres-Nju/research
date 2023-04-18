fn to_fixedpoint_16_6(f: f64) -> i64 {
    (f * 65536.0) as i64
}
