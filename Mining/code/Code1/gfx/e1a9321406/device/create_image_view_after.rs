    fn create_image_view(
        &self,
        image: &n::Image,
        kind: image::ViewKind,
        format: format::Format,
        swizzle: format::Swizzle,
        range: image::SubresourceRange,
    ) -> Result<n::ImageView, image::ViewError> {
        let mtl_format = match self.private_caps.map_format_with_swizzle(format, swizzle) {
            Some(f) => f,
            None => {
                error!("failed to swizzle format {:?} with {:?}", format, swizzle);
                return Err(image::ViewError::BadFormat);
            },
        };

        let full_range = image::SubresourceRange {
            aspects: image.format_desc.aspects,
            levels: 0 .. image.raw.mipmap_level_count() as image::Level,
            layers: 0 .. image.kind.num_layers(),
        };
        let view = if
            mtl_format == image.mtl_format &&
            //kind == image::ViewKind::D2 && //TODO: find a better way to check this
            swizzle == format::Swizzle::NO &&
            range == full_range &&
            match (kind, image.kind) {
                (image::ViewKind::D1, image::Kind::D1(..)) |
                (image::ViewKind::D2, image::Kind::D2(..)) |
                (image::ViewKind::D3, image::Kind::D3(..)) => true,
                (image::ViewKind::D1Array, image::Kind::D1(_, layers)) if layers > 1 => true,
                (image::ViewKind::D2Array, image::Kind::D2(_, _, layers, _)) if layers > 1 => true,
                (_, _) => false, //TODO: expose more choices here?
            }
        {
            // Some images are marked as framebuffer-only, and we can't create aliases of them.
            // Also helps working around Metal bugs with aliased array textures.
            image.raw.clone()
        } else {
            image.raw.new_texture_view_from_slice(
                mtl_format,
                conv::map_texture_type(kind),
                NSRange {
                    location: range.levels.start as _,
                    length: (range.levels.end - range.levels.start) as _,
                },
                NSRange {
                    location: range.layers.start as _,
                    length: (range.layers.end - range.layers.start) as _,
                },
            )
        };

        Ok(n::ImageView { raw: view, mtl_format })
    }
