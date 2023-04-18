    fn view_image_as_shader_resource(
        &self, info: ViewInfo
    ) -> Result<d3d12::D3D12_CPU_DESCRIPTOR_HANDLE, image::ViewError> {
        #![allow(non_snake_case)]

        // Depth-stencil formats can't be used for SRVs.
        let format = match info.format {
            dxgiformat::DXGI_FORMAT_D16_UNORM => dxgiformat::DXGI_FORMAT_R16_UNORM,
            dxgiformat::DXGI_FORMAT_D32_FLOAT => dxgiformat::DXGI_FORMAT_R32_FLOAT,
            format => format,
        };

        let mut desc = d3d12::D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: format,
            ViewDimension: 0,
            Shader4ComponentMapping: 0x1688, // TODO: map swizzle
            u: unsafe { mem::zeroed() },
        };

        let MostDetailedMip = info.range.levels.start as _;
        let MipLevels = (info.range.levels.end - info.range.levels.start) as _;
        let FirstArraySlice = info.range.layers.start as _;
        let ArraySize = (info.range.layers.end - info.range.layers.start) as _;

        assert!(info.range.layers.end <= info.kind.num_layers());
        let is_msaa = info.kind.num_samples() > 1;
        let is_cube = info.flags.contains(image::StorageFlags::CUBE_VIEW);

        match info.view_kind {
            image::ViewKind::D1 => {
                desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE1D;
                *unsafe{ desc.u.Texture1D_mut() } = d3d12::D3D12_TEX1D_SRV {
                    MostDetailedMip,
                    MipLevels,
                    ResourceMinLODClamp: 0.0,
                }
            }
            image::ViewKind::D1Array => {
                desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE1DARRAY;
                *unsafe{ desc.u.Texture1DArray_mut() } = d3d12::D3D12_TEX1D_ARRAY_SRV {
                    MostDetailedMip,
                    MipLevels,
                    FirstArraySlice,
                    ArraySize,
                    ResourceMinLODClamp: 0.0,
                }
            }
            image::ViewKind::D2 if is_msaa => {
                desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE2DMS;
                *unsafe{ desc.u.Texture2DMS_mut() } = d3d12::D3D12_TEX2DMS_SRV {
                    UnusedField_NothingToDefine: 0,
                }
            }
            image::ViewKind::D2 => {
                desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE2D;
                *unsafe{ desc.u.Texture2D_mut() } = d3d12::D3D12_TEX2D_SRV {
                    MostDetailedMip,
                    MipLevels,
                    PlaneSlice: 0, //TODO
                    ResourceMinLODClamp: 0.0,
                }
            }
            image::ViewKind::D2Array if is_msaa => {
                desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE2DMSARRAY;
                *unsafe{ desc.u.Texture2DMSArray_mut() } = d3d12::D3D12_TEX2DMS_ARRAY_SRV {
                    FirstArraySlice,
                    ArraySize,
                }
            }
            image::ViewKind::D2Array => {
                desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE2DARRAY;
                *unsafe{ desc.u.Texture2DArray_mut() } = d3d12::D3D12_TEX2D_ARRAY_SRV {
                    MostDetailedMip,
                    MipLevels,
                    FirstArraySlice,
                    ArraySize,
                    PlaneSlice: 0, //TODO
                    ResourceMinLODClamp: 0.0,
                }
            }
            image::ViewKind::D3 => {
                desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURE3D;
                *unsafe{ desc.u.Texture3D_mut() } = d3d12::D3D12_TEX3D_SRV {
                    MostDetailedMip,
                    MipLevels,
                    ResourceMinLODClamp: 0.0,
                }
            }
            image::ViewKind::Cube if is_cube => {
                desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURECUBE;
                *unsafe{ desc.u.TextureCube_mut() } = d3d12::D3D12_TEXCUBE_SRV {
                    MostDetailedMip,
                    MipLevels,
                    ResourceMinLODClamp: 0.0,
                }
            }
            image::ViewKind::CubeArray if is_cube => {
                assert_eq!(0, ArraySize % 6);
                desc.ViewDimension = d3d12::D3D12_SRV_DIMENSION_TEXTURECUBEARRAY;
                *unsafe{ desc.u.TextureCubeArray_mut() } = d3d12::D3D12_TEXCUBE_ARRAY_SRV {
                    MostDetailedMip,
                    MipLevels,
                    First2DArrayFace: FirstArraySlice,
                    NumCubes: ArraySize / 6,
                    ResourceMinLODClamp: 0.0,
                }
            }
            image::ViewKind::Cube |
            image::ViewKind::CubeArray => {
                error!("Cube views are not supported for the image, kind: {:?}", info.kind);
                return Err(image::ViewError::BadKind)
            }
        }

        let handle = self.srv_pool.lock().unwrap().alloc_handles(1).cpu;
        unsafe {
            self.raw.clone().CreateShaderResourceView(info.resource, &desc, handle);
        }

        Ok(handle)
    }
