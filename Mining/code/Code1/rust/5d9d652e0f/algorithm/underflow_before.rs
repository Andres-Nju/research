fn underflow<T: RawFloat>(x: Big, v: Big, rem: Big) -> T {
    if x < Big::from_u64(T::min_sig()) {
        let q = num::to_u64(&x);
        let z = rawfp::encode_subnormal(q);
        return round_by_remainder(v, rem, q, z);
    }
    // Ratio isn't an in-range significand with the minimum exponent, so we need to round off
    // excess bits and adjust the exponent accordingly. The real value now looks like this:
    //
    //        x        lsb
    // /--------------\/
    // 1010101010101010.10101010101010 * 2^k
    // \-----/\-------/ \------------/
    //    q     trunc.    (represented by rem)
    //
    // Therefore, when the rounded-off bits are != 0.5 ULP, they decide the rounding
    // on their own. When they are equal and the remainder is non-zero, the value still
    // needs to be rounded up. Only when the rounded off bits are 1/2 and the remainer
    // is zero, we have a half-to-even situation.
    let bits = x.bit_length();
    let lsb = bits - T::sig_bits() as usize;
    let q = num::get_bits(&x, lsb, bits);
    let k = T::min_exp_int() + lsb as i16;
    let z = rawfp::encode_normal(Unpacked::new(q, k));
    let q_even = q % 2 == 0;
    match num::compare_with_half_ulp(&x, lsb) {
        Greater => next_float(z),
        Less => z,
        Equal if rem.is_zero() && q_even => z,
        Equal => next_float(z),
    }
}
