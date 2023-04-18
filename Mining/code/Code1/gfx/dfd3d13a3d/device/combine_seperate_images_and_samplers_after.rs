    fn combine_separate_images_and_samplers(
        &self,
        ast: &mut spirv::Ast<glsl::Target>,
        desc_remap_data: &mut n::DescRemapData,
        nb_map: &mut FastHashMap<String, pso::DescriptorBinding>,
    ) {
        let mut id_map = FastHashMap::<u32, (pso::DescriptorSetIndex, pso::DescriptorBinding)>::default();
        let res = ast.get_shader_resources().unwrap();
        self.populate_id_map(ast, &mut id_map, &res.separate_images);
        self.populate_id_map(ast, &mut id_map, &res.separate_samplers);

        for cis in ast.get_combined_image_samplers().unwrap() {
            let (set, binding) = id_map.get(&cis.image_id).unwrap();
            let nb = desc_remap_data.reserve_binding(n::BindingTypes::Images);
            desc_remap_data.insert_missing_binding(
                nb,
                n::BindingTypes::Images,
                *set,
                *binding,
            );
            let (set, binding) = id_map.get(&cis.sampler_id).unwrap();
            desc_remap_data.insert_missing_binding(
                nb,
                n::BindingTypes::Images,
                *set,
                *binding,
            );

            let new_name = "GFX_HAL_COMBINED_SAMPLER".to_owned()
                + "_" + &cis.sampler_id.to_string()
                + "_" + &cis.image_id.to_string()
                + "_" + &cis.combined_id.to_string() ;
            ast.set_name(cis.combined_id, &new_name).unwrap();
            if self.share.legacy_features.contains(LegacyFeatures::EXPLICIT_LAYOUTS_IN_SHADER) {
                ast.set_decoration(cis.combined_id, spirv::Decoration::Binding, nb).unwrap()
            } else {
                ast.unset_decoration(cis.combined_id, spirv::Decoration::Binding).unwrap();
                assert!(nb_map.insert(new_name, nb).is_none())
            }
            ast.unset_decoration(cis.combined_id, spirv::Decoration::DescriptorSet).unwrap();
        }
    }
