    fn is_supported(ext: &WebGLExtensions) -> bool {
        ext.supports_any_gl_extension(&["GL_OES_texture_float",
                                        "GL_ARB_texture_float",
                                        "GL_EXT_color_buffer_float"])
    }
