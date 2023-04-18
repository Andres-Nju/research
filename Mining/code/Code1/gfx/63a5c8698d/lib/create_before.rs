    pub fn create(name: &str, version: u32) -> Self {
        // TODO: return errors instead of panic
        let entry = VK_ENTRY.as_ref().expect("Unable to load Vulkan entry points");

        let app_name = CString::new(name).unwrap();
        let app_info = vk::ApplicationInfo {
            s_type: vk::StructureType::ApplicationInfo,
            p_next: ptr::null(),
            p_application_name: app_name.as_ptr(),
            application_version: version,
            p_engine_name: b"gfx-rs\0".as_ptr() as *const _,
            engine_version: 1,
            api_version: 0, //TODO
        };

        let instance_extensions = entry
            .enumerate_instance_extension_properties()
            .expect("Unable to enumerate instance extensions");

        let instance_layers = entry
            .enumerate_instance_layer_properties()
            .expect("Unable to enumerate instance layers");

        // Check our xtensions against the available extensions
        let extensions = SURFACE_EXTENSIONS
            .iter()
            .chain(EXTENSIONS.iter())
            .filter_map(|&ext| {
                instance_extensions
                    .iter()
                    .find(|inst_ext| unsafe {
                        CStr::from_ptr(inst_ext.extension_name.as_ptr()) ==
                            CStr::from_ptr(ext.as_ptr() as *const _)
                    })
                    .map(|_| ext)
                    .or_else(|| {
                        warn!("Unable to find extension: {}", ext);
                        None
                    })
            })
            .collect::<Vec<&str>>();

        // Check requested layers against the available layers
        let layers = LAYERS
            .iter()
            .filter_map(|&layer| {
                instance_layers
                    .iter()
                    .find(|inst_layer| unsafe {
                        CStr::from_ptr(inst_layer.layer_name.as_ptr()) ==
                            CStr::from_ptr(layer.as_ptr() as *const _)
                    })
                    .map(|_| layer)
                    .or_else(|| {
                        warn!("Unable to find layer: {}", layer);
                        None
                    })
            })
            .collect::<Vec<&str>>();

        let instance = {
            let cstrings = layers
                .iter()
                .chain(extensions.iter())
                .map(|&s| CString::new(s).unwrap())
                .collect::<Vec<_>>();

            let str_pointers = cstrings
                .iter()
                .map(|s| s.as_ptr())
                .collect::<Vec<_>>();

            let create_info = vk::InstanceCreateInfo {
                s_type: vk::StructureType::InstanceCreateInfo,
                p_next: ptr::null(),
                flags: vk::InstanceCreateFlags::empty(),
                p_application_info: &app_info,
                enabled_layer_count: layers.len() as _,
                pp_enabled_layer_names: str_pointers.as_ptr(),
                enabled_extension_count: extensions.len() as _,
                pp_enabled_extension_names: str_pointers[layers.len()..].as_ptr(),
            };

            unsafe {
                entry.create_instance(&create_info, None)
            }.expect("Unable to create Vulkan instance")
        };

        #[cfg(debug_assertions)]
        let debug_report = {
            let ext = ext::DebugReport::new(entry, &instance).unwrap();
            let info = vk::DebugReportCallbackCreateInfoEXT {
                s_type: vk::StructureType::DebugReportCallbackCreateInfoExt,
                p_next: ptr::null(),
                flags: vk::DEBUG_REPORT_WARNING_BIT_EXT |
                       vk::DEBUG_REPORT_PERFORMANCE_WARNING_BIT_EXT |
                       vk::DEBUG_REPORT_ERROR_BIT_EXT,
                pfn_callback: callback,
                p_user_data: ptr::null_mut(),
            };
            let handle = unsafe {
                ext.create_debug_report_callback_ext(&info, None)
            }.unwrap();
            Some((ext, handle))
        };
        #[cfg(not(debug_assertions))]
        let debug_report = None;

        Instance {
            raw: Arc::new(RawInstance(instance, debug_report)),
            extensions,
        }
    }
