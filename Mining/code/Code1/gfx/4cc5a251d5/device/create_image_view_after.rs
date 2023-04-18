    unsafe fn create_image_view(
        &self,
        image: &r::Image,
        view_kind: image::ViewKind,
        format: format::Format,
        _swizzle: format::Swizzle,
        range: image::SubresourceRange,
    ) -> Result<r::ImageView, image::ViewError> {
        let image = image.expect_bound();
        let is_array = image.kind.num_layers() > 1;
        let mip_levels = (range.levels.start, range.levels.end);
        let layers = (range.layers.start, range.layers.end);

        let info = ViewInfo {
            resource: image.resource,
            kind: image.kind,
            caps: image.view_caps,
            // D3D12 doesn't allow looking at a single slice of an array as a non-array
            view_kind: if is_array && view_kind == image::ViewKind::D2 {
                image::ViewKind::D2Array
            } else if is_array && view_kind == image::ViewKind::D1 {
                image::ViewKind::D1Array
            } else {
                view_kind
            },
            format: conv::map_format(format).ok_or(image::ViewError::BadFormat(format))?,
            range,
        };

        Ok(r::ImageView {
            resource: image.resource,
            handle_srv: if image
                .usage
                .intersects(image::Usage::SAMPLED | image::Usage::INPUT_ATTACHMENT)
            {
                Some(self.view_image_as_shader_resource(info.clone())?)
            } else {
                None
            },
            handle_rtv: if image.usage.contains(image::Usage::COLOR_ATTACHMENT) {
                Some(self.view_image_as_render_target(info.clone())?)
            } else {
                None
            },
            handle_uav: if image.usage.contains(image::Usage::STORAGE) {
                Some(self.view_image_as_storage(info.clone())?)
            } else {
                None
            },
            handle_dsv: if image.usage.contains(image::Usage::DEPTH_STENCIL_ATTACHMENT) {
                Some(
                    self.view_image_as_depth_stencil(ViewInfo {
                        format: conv::map_format_dsv(format.base_format().0)
                            .ok_or(image::ViewError::BadFormat(format))?,
                        ..info
                    })?,
                )
            } else {
                None
            },
            dxgi_format: image.default_view_format.unwrap(),
            num_levels: image.descriptor.MipLevels as image::Level,
            mip_levels,
            layers,
            kind: info.kind,
        })
    }
