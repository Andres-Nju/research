    fn is_supported(ext: &WebGLExtensions) -> bool {
        ext.supports_any_gl_extension(&["GL_OES_texture_half_float",
                                        "GL_ARB_half_float_pixel",
                                        "GL_NV_half_float"])
    }
