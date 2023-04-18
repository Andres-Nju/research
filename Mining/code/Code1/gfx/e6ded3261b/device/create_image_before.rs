    fn create_image(
        &self,
        kind: image::Kind,
        mip_levels: image::Level,
        format: format::Format,
        tiling: image::Tiling,
        usage: image::Usage,
        flags: image::StorageFlags,
    ) -> Result<UnboundImage, image::CreationError> {
        assert!(mip_levels <= kind.num_levels());

        let base_format = format.base_format();
        let format_desc = base_format.0.desc();
        let bytes_per_block = (format_desc.bits / 8) as _;
        let block_dim = format_desc.dim;
        let extent = kind.extent();

        let format_properties = &self.format_properties[format as usize];
        let (layout, features) = match tiling {
            image::Tiling::Optimal => (d3d12::D3D12_TEXTURE_LAYOUT_UNKNOWN, format_properties.optimal_tiling),
            image::Tiling::Linear => (d3d12::D3D12_TEXTURE_LAYOUT_ROW_MAJOR, format_properties.linear_tiling),
        };

        let desc = d3d12::D3D12_RESOURCE_DESC {
            Dimension: match kind {
                image::Kind::D1(..) => d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE1D,
                image::Kind::D2(..) => d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                image::Kind::D3(..) => d3d12::D3D12_RESOURCE_DIMENSION_TEXTURE3D,
            },
            Alignment: 0,
            Width: extent.width as _,
            Height: extent.height as _,
            DepthOrArraySize: if extent.depth > 1 {
                extent.depth as _
            } else {
                kind.num_layers() as _
            },
            MipLevels: mip_levels as _,
            Format: match conv::map_format(format) {
                Some(format) => format,
                None => return Err(image::CreationError::Format(format)),
            },
            SampleDesc: dxgitype::DXGI_SAMPLE_DESC {
                Count: kind.num_samples() as _,
                Quality: 0,
            },
            Layout: layout,
            Flags: conv::map_image_flags(usage, features),
        };

        let alloc_info = unsafe {
            self.raw.clone().GetResourceAllocationInfo(0, 1, &desc)
        };

        let type_mask_shift = if self.private_caps.heterogeneous_resource_heaps {
            MEM_TYPE_UNIVERSAL_SHIFT
        } else if usage.can_target() {
            MEM_TYPE_TARGET_SHIFT
        } else {
            MEM_TYPE_IMAGE_SHIFT
        };

        Ok(UnboundImage {
            dsv_format: conv::map_format_dsv(base_format.0)
                .unwrap_or(desc.Format),
            desc,
            requirements: memory::Requirements {
                size: alloc_info.SizeInBytes,
                alignment: alloc_info.Alignment,
                type_mask: MEM_TYPE_MASK << type_mask_shift,
            },
            format,
            kind,
            usage,
            tiling,
            storage_flags: flags,
            bytes_per_block,
            block_dim,
            num_levels: mip_levels,
        })
    }
