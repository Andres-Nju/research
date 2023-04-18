pub(crate) fn query_all(gl: &GlContainer) -> (Info, Features, LegacyFeatures, Limits, PrivateCaps) {
    use self::Requirement::*;
    let info = Info::get(gl);
    let max_texture_size = get_usize(gl, glow::MAX_TEXTURE_SIZE).unwrap_or(64) as u32;
    let max_samples = get_usize(gl, glow::MAX_SAMPLES).unwrap_or(8);
    let max_samples_mask = (max_samples * 2 - 1) as u8;

    let mut limits = Limits {
        max_image_1d_size: max_texture_size,
        max_image_2d_size: max_texture_size,
        max_image_3d_size: max_texture_size,
        max_image_cube_size: max_texture_size,
        max_image_array_layers: get_usize(gl, glow::MAX_ARRAY_TEXTURE_LAYERS).unwrap_or(1) as u16,
        max_texel_elements: get_usize(gl, glow::MAX_TEXTURE_BUFFER_SIZE).unwrap_or(0),
        max_viewports: 1,
        optimal_buffer_copy_offset_alignment: 1,
        optimal_buffer_copy_pitch_alignment: 1,
        min_texel_buffer_offset_alignment: 1,   // TODO
        min_uniform_buffer_offset_alignment: 1, // TODO
        min_storage_buffer_offset_alignment: 1, // TODO
        framebuffer_color_samples_count: max_samples_mask,
        non_coherent_atom_size: 1,
        ..Limits::default()
    };

    if info.is_supported(&[Core(4, 0), Ext("GL_ARB_tessellation_shader")]) {
        limits.max_patch_size = get_usize(gl, glow::MAX_PATCH_VERTICES).unwrap_or(0) as _;
    }
    if info.is_supported(&[Core(4, 1)]) {
        // TODO: extension
        limits.max_viewports = get_usize(gl, glow::MAX_VIEWPORTS).unwrap_or(0);
    }

    if false
        && info.is_supported(&[
            //TODO: enable when compute is implemented
            Core(4, 3),
            Ext("GL_ARB_compute_shader"),
        ])
    {
        for (i, (count, size)) in limits
            .max_compute_work_group_count
            .iter_mut()
            .zip(limits.max_compute_work_group_size.iter_mut())
            .enumerate()
        {
            unsafe {
                *count =
                    gl.get_parameter_indexed_i32(glow::MAX_COMPUTE_WORK_GROUP_COUNT, i as _) as u32;
                *size =
                    gl.get_parameter_indexed_i32(glow::MAX_COMPUTE_WORK_GROUP_SIZE, i as _) as u32;
            }
        }
    }

    let mut features = Features::empty();
    let mut legacy = LegacyFeatures::empty();

    if info.is_supported(&[
        Core(4, 6),
        Ext("GL_ARB_texture_filter_anisotropic"),
        Ext("GL_EXT_texture_filter_anisotropic"),
    ]) {
        features |= Features::SAMPLER_ANISOTROPY;
    }
    if info.is_supported(&[Core(4, 2)]) {
        legacy |= LegacyFeatures::EXPLICIT_LAYOUTS_IN_SHADER;
    }
    if info.is_supported(&[Core(3, 3), Es(3, 0), Ext("GL_ARB_instanced_arrays")]) {
        features |= Features::INSTANCE_RATE;
    }
    if info.is_supported(&[Core(3, 3)]) {
        // TODO: extension
        features |= Features::SAMPLER_MIP_LOD_BIAS;
    }

    // TODO
    if false && info.is_supported(&[Core(4, 3), Es(3, 1)]) {
        // TODO: extension
        legacy |= LegacyFeatures::INDIRECT_EXECUTION;
    }
    if info.is_supported(&[Core(3, 1), Es(3, 0), Ext("GL_ARB_draw_instanced")]) {
        legacy |= LegacyFeatures::DRAW_INSTANCED;
    }
    if info.is_supported(&[Core(4, 2), Ext("GL_ARB_base_instance")]) {
        legacy |= LegacyFeatures::DRAW_INSTANCED_BASE;
    }
    if info.is_supported(&[Core(3, 2)]) {
        // TODO: extension
        legacy |= LegacyFeatures::DRAW_INDEXED_BASE;
    }
    if info.is_supported(&[Core(3, 1), Es(3, 0)]) {
        // TODO: extension
        legacy |= LegacyFeatures::DRAW_INDEXED_INSTANCED;
    }
    if info.is_supported(&[Core(3, 2)]) {
        // TODO: extension
        legacy |= LegacyFeatures::DRAW_INDEXED_INSTANCED_BASE_VERTEX;
    }
    if info.is_supported(&[Core(4, 2)]) {
        // TODO: extension
        legacy |= LegacyFeatures::DRAW_INDEXED_INSTANCED_BASE;
    }
    if info.is_supported(&[
        Core(3, 2),
        Es(3, 2),
        Ext("GL_ARB_draw_elements_base_vertex"),
    ]) {
        legacy |= LegacyFeatures::VERTEX_BASE;
    }
    if info.is_supported(&[Core(3, 2), Ext("GL_ARB_framebuffer_sRGB")]) {
        legacy |= LegacyFeatures::SRGB_COLOR;
    }
    if info.is_supported(&[Core(3, 1), Es(3, 0), Ext("GL_ARB_uniform_buffer_object")]) {
        legacy |= LegacyFeatures::CONSTANT_BUFFER;
    }
    if info.is_supported(&[Core(4, 0)]) {
        // TODO: extension
        legacy |= LegacyFeatures::UNORDERED_ACCESS_VIEW;
    }
    if info.is_supported(&[
        Core(3, 1),
        Es(3, 0),
        Ext("GL_ARB_copy_buffer"),
        Ext("GL_NV_copy_buffer"),
    ]) {
        legacy |= LegacyFeatures::COPY_BUFFER;
    }
    if info.is_supported(&[Core(3, 3), Es(3, 0), Ext("GL_ARB_sampler_objects")]) {
        legacy |= LegacyFeatures::SAMPLER_OBJECTS;
    }
    if info.is_supported(&[Core(3, 3)]) {
        // TODO: extension
        legacy |= LegacyFeatures::SAMPLER_BORDER_COLOR;
    }
    if info.is_supported(&[Core(3, 3), Es(3, 0)]) {
        legacy |= LegacyFeatures::INSTANCED_ATTRIBUTE_BINDING;
    }

    let per_draw_buffer_blending =
        info.is_supported(&[Core(4, 0), Es(3, 2), Ext("GL_EXT_draw_buffers2")])
        && !info.is_webgl();
    if per_draw_buffer_blending {
        features |= Features::INDEPENDENT_BLENDING;
    }

    let emulate_map = info.version.is_embedded;

    let private = PrivateCaps {
        vertex_array: info.is_supported(&[Core(3, 0), Es(3, 0), Ext("GL_ARB_vertex_array_object")]),
        // TODO && gl.GenVertexArrays.is_loaded(),
        framebuffer: info.is_supported(&[Core(3, 0), Es(2, 0), Ext("GL_ARB_framebuffer_object")]),
        // TODO && gl.GenFramebuffers.is_loaded(),
        framebuffer_texture: info.is_supported(&[Core(3, 0)]), //TODO: double check
        index_buffer_role_change: !info.is_webgl(),
        image_storage: info.is_supported(&[Core(4, 2), Ext("GL_ARB_texture_storage")]),
        buffer_storage: info.is_supported(&[Core(4, 4), Ext("GL_ARB_buffer_storage")]),
        clear_buffer: info.is_supported(&[Core(3, 0), Es(3, 0)]),
        program_interface: info.is_supported(&[Core(4, 3), Ext("GL_ARB_program_interface_query")]),
        frag_data_location: !info.version.is_embedded,
        sync: !info.is_webgl() && info.is_supported(&[Core(3, 2), Es(3, 0), Ext("GL_ARB_sync")]), // TODO
        map: !info.version.is_embedded, //TODO: OES extension
        sampler_anisotropy_ext: !info
            .is_supported(&[Core(4, 6), Ext("GL_ARB_texture_filter_anisotropic")])
            && info.is_supported(&[Ext("GL_EXT_texture_filter_anisotropic")]),
        emulate_map, // TODO
        depth_range_f64_precision: !info.version.is_embedded, // TODO
        draw_buffers: info.is_supported(&[Core(2, 0), Es(3, 0)]),
        per_draw_buffer_blending,
    };

    (info, features, legacy, limits, private)
}
