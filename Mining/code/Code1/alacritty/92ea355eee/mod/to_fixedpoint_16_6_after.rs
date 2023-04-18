fn to_fixedpoint_16_6(f: f64) -> c_long {
    (f * 65536.0) as c_long
}
