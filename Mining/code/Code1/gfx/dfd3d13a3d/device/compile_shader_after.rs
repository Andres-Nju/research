    fn compile_shader(
        &self,
        point: &pso::EntryPoint<B>,
        stage: pso::Stage,
        desc_remap_data: &mut n::DescRemapData,
        name_binding_map: &mut FastHashMap<String, pso::DescriptorBinding>,
    ) -> n::Shader {
        assert_eq!(point.entry, "main");
        match *point.module {
            n::ShaderModule::Raw(raw) => {
                debug!("Can't remap bindings for raw shaders. Assuming they are already rebound.");
                raw
            }
            n::ShaderModule::Spirv(ref spirv) => {
                let mut ast = self.parse_spirv(spirv).unwrap();

                self.specialize_ast(&mut ast, point.specialization).unwrap();
                self.remap_bindings(&mut ast, desc_remap_data, name_binding_map);
                self.combine_separate_images_and_samplers(&mut ast, desc_remap_data, name_binding_map);

                let glsl = self.translate_spirv(&mut ast).unwrap();
                info!("Generated:\n{:?}", glsl);
                let shader = match self.create_shader_module_from_source(glsl.as_bytes(), stage).unwrap() {
                    n::ShaderModule::Raw(raw) => raw,
                    _ => panic!("Unhandled")
                };

                shader
            }
        }
    }
