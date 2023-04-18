    pub(crate) fn new(shared: Arc<Shared>) -> Self {
        let device = shared.device.lock();

        let version: NSOperatingSystemVersion = unsafe {
            let process_info: *mut Object = msg_send![class!(NSProcessInfo), processInfo];
            msg_send![process_info, operatingSystemVersion]
        };

        let major = version.major as u32;
        let minor = version.minor as u32;
        let os_is_mac = device.supports_feature_set(MTLFeatureSet::macOS_GPUFamily1_v1);

        let private_caps = {
            PrivateCapabilities {
                os_is_mac,
                os_version: (major as u32, minor as u32),
                msl_version: if os_is_mac {
                    if PrivateCapabilities::version_at_least(major, minor, 10, 13) {
                        MTLLanguageVersion::V2_0
                    } else if PrivateCapabilities::version_at_least(major, minor, 10, 12) {
                        MTLLanguageVersion::V1_2
                    } else if PrivateCapabilities::version_at_least(major, minor, 10, 11) {
                        MTLLanguageVersion::V1_1
                    } else {
                        MTLLanguageVersion::V1_0
                    }
                } else if PrivateCapabilities::version_at_least(major, minor, 11, 0) {
                    MTLLanguageVersion::V2_0
                } else if PrivateCapabilities::version_at_least(major, minor, 10, 0) {
                    MTLLanguageVersion::V1_2
                } else if PrivateCapabilities::version_at_least(major, minor, 9, 0) {
                    MTLLanguageVersion::V1_1
                } else {
                    MTLLanguageVersion::V1_0
                },
                exposed_queues: 1,
                resource_heaps: Self::supports_any(&device, RESOURCE_HEAP_SUPPORT),
                argument_buffers: Self::supports_any(&device, ARGUMENT_BUFFER_SUPPORT) && false, //TODO
                shared_textures: !os_is_mac,
                base_instance: Self::supports_any(&device, BASE_INSTANCE_SUPPORT),
                dual_source_blending: Self::supports_any(&device, DUAL_SOURCE_BLEND_SUPPORT),
                format_depth24_stencil8: os_is_mac && device.d24_s8_supported(),
                format_depth32_stencil8_filter: os_is_mac,
                format_depth32_stencil8_none: !os_is_mac,
                format_min_srgb_channels: if os_is_mac {4} else {1},
                format_b5: !os_is_mac,
                format_bc: os_is_mac,
                format_eac_etc: !os_is_mac,
                format_astc: Self::supports_any(&device, ASTC_PIXEL_FORMAT_FEATURES),
                format_r8unorm_srgb_all: Self::supports_any(&device, R8UNORM_SRGB_ALL),
                format_r8unorm_srgb_no_write: !Self::supports_any(&device, R8UNORM_SRGB_ALL) && !os_is_mac,
                format_r8snorm_all: !Self::supports_any(&device, R8SNORM_NO_RESOLVE),
                format_r16_norm_all: os_is_mac,
                format_rg8unorm_srgb_all: Self::supports_any(&device, RG8UNORM_SRGB_NO_WRITE),
                format_rg8unorm_srgb_no_write: !Self::supports_any(&device, RG8UNORM_SRGB_NO_WRITE) && !os_is_mac,
                format_rg8snorm_all: !Self::supports_any(&device, RG8SNORM_NO_RESOLVE),
                format_r32_all: !Self::supports_any(&device, &[MTLFeatureSet::iOS_GPUFamily1_v1, MTLFeatureSet::iOS_GPUFamily2_v1]),
                format_r32_no_write: Self::supports_any(&device, &[MTLFeatureSet::iOS_GPUFamily1_v1, MTLFeatureSet::iOS_GPUFamily2_v1]),
                format_r32float_no_write_no_filter: Self::supports_any(&device, &[MTLFeatureSet::iOS_GPUFamily1_v1, MTLFeatureSet::iOS_GPUFamily2_v1]) && !os_is_mac,
                format_r32float_no_filter: !Self::supports_any(&device, &[MTLFeatureSet::iOS_GPUFamily1_v1, MTLFeatureSet::iOS_GPUFamily2_v1]) && !os_is_mac,
                format_r32float_all: os_is_mac,
                format_rgba8_srgb_all: Self::supports_any(&device, RGBA8_SRGB),
                format_rgba8_srgb_no_write: !Self::supports_any(&device, RGBA8_SRGB),
                format_rgb10a2_unorm_all: Self::supports_any(&device, RGB10A2UNORM_ALL),
                format_rgb10a2_unorm_no_write: !Self::supports_any(&device, RGB10A2UNORM_ALL),
                format_rgb10a2_uint_color: !Self::supports_any(&device, RGB10A2UINT_COLOR_WRITE),
                format_rgb10a2_uint_color_write: Self::supports_any(&device, RGB10A2UINT_COLOR_WRITE),
                format_rg11b10_all: Self::supports_any(&device, RG11B10FLOAT_ALL),
                format_rg11b10_no_write: !Self::supports_any(&device, RG11B10FLOAT_ALL),
                format_rgb9e5_all: Self::supports_any(&device, RGB9E5FLOAT_ALL),
                format_rgb9e5_no_write: !Self::supports_any(&device, RGB9E5FLOAT_ALL) && !os_is_mac,
                format_rgb9e5_filter_only: os_is_mac,
                format_rg32_color: Self::supports_any(&device, &[MTLFeatureSet::iOS_GPUFamily1_v1, MTLFeatureSet::iOS_GPUFamily2_v1]),
                format_rg32_color_write: !Self::supports_any(&device, &[MTLFeatureSet::iOS_GPUFamily1_v1, MTLFeatureSet::iOS_GPUFamily2_v1]),
                format_rg32float_all: os_is_mac,
                format_rg32float_color_blend: Self::supports_any(&device, &[MTLFeatureSet::iOS_GPUFamily1_v1, MTLFeatureSet::iOS_GPUFamily2_v1]),
                format_rg32float_no_filter: !os_is_mac && !Self::supports_any(&device, &[MTLFeatureSet::iOS_GPUFamily1_v1, MTLFeatureSet::iOS_GPUFamily2_v1]),
                format_rgba32int_color: Self::supports_any(&device, &[MTLFeatureSet::iOS_GPUFamily1_v1, MTLFeatureSet::iOS_GPUFamily2_v1]),
                format_rgba32int_color_write: !Self::supports_any(&device, &[MTLFeatureSet::iOS_GPUFamily1_v1, MTLFeatureSet::iOS_GPUFamily2_v1]),
                format_rgba32float_color: Self::supports_any(&device, &[MTLFeatureSet::iOS_GPUFamily1_v1, MTLFeatureSet::iOS_GPUFamily2_v1]),
                format_rgba32float_color_write: !Self::supports_any(&device, &[MTLFeatureSet::iOS_GPUFamily1_v1, MTLFeatureSet::iOS_GPUFamily2_v1]) && !os_is_mac,
                format_rgba32float_all: os_is_mac,
                format_depth16unorm: Self::supports_any(&device, &[MTLFeatureSet::macOS_GPUFamily1_v2, MTLFeatureSet::macOS_GPUFamily1_v3]),
                format_depth32float_filter: Self::supports_any(&device, &[MTLFeatureSet::macOS_GPUFamily1_v1, MTLFeatureSet::macOS_GPUFamily1_v2, MTLFeatureSet::macOS_GPUFamily1_v3]),
                format_depth32float_none: !Self::supports_any(&device, &[MTLFeatureSet::macOS_GPUFamily1_v1, MTLFeatureSet::macOS_GPUFamily1_v2, MTLFeatureSet::macOS_GPUFamily1_v3]),
                format_bgr10a2_all: Self::supports_any(&device, BGR10A2_ALL),
                format_bgr10a2_no_write: !Self::supports_any(&device, &[MTLFeatureSet::macOS_GPUFamily1_v3]),
                max_buffers_per_stage: 31,
                max_textures_per_stage: if os_is_mac {128} else {31},
                max_samplers_per_stage: 16,
                buffer_alignment: if os_is_mac {256} else {64},
                max_buffer_size: if Self::supports_any(&device, &[MTLFeatureSet::macOS_GPUFamily1_v2, MTLFeatureSet::macOS_GPUFamily1_v3]) {
                    1 << 30 // 1GB on macOS 1.2 and up
                } else {
                    1 << 28 // 256MB otherwise
                },
                max_texture_size: 4096, //TODO
            }
        };

        PhysicalDevice {
            shared:  shared.clone(),
            memory_types: [
                hal::MemoryType { // PRIVATE
                    properties: Properties::DEVICE_LOCAL,
                    heap_index: 0,
                },
                hal::MemoryType { // SHARED
                    properties: Properties::CPU_VISIBLE | Properties::COHERENT,
                    heap_index: 1,
                },
                hal::MemoryType { // MANAGED_UPLOAD
                    properties: Properties::DEVICE_LOCAL | Properties::CPU_VISIBLE,
                    heap_index: 1,
                },
                hal::MemoryType { // MANAGED_DOWNLOAD
                    properties: Properties::DEVICE_LOCAL | Properties::CPU_VISIBLE | Properties::CPU_CACHED,
                    heap_index: 1,
                },
            ],
            private_caps,
        }
    }
