fn format_to_glpixel(format: NewFormat) -> GLenum {
    use core::format::SurfaceType as S;
    use core::format::ChannelType as C;
    let (r, rg, rgb, rgba, bgra) = match format.1 {
        C::Int | C::Uint => (gl::RED_INTEGER, gl::RG_INTEGER, gl::RGB_INTEGER, gl::RGBA_INTEGER, gl::BGRA_INTEGER),
        _ => (gl::RED, gl::RG, gl::RGB, gl::RGBA, gl::BGRA),
    };
    match format.0 {
        S::R8 | S::R16 | S::R32=> r,
        S::R4_G4 | S::R8_G8 | S::R16_G16 | S::R32_G32 => rg,
        S::R16_G16_B16 | S::R32_G32_B32 | S::R5_G6_B5 | S::R11_G11_B10 => rgb,
        S::R8_G8_B8_A8 | S::R16_G16_B16_A16 | S::R32_G32_B32_A32 |
        S::R4_G4_B4_A4 | S::R5_G5_B5_A1 | S::R10_G10_B10_A2 => rgba,
        S::D24_S8 => gl::DEPTH_STENCIL,
        S::D16 | S::D24 | S::D32 => gl::DEPTH_COMPONENT,
        S::B8_G8_R8_A8 => bgra,
    }
}
