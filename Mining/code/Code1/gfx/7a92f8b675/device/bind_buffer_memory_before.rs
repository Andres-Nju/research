    unsafe fn bind_buffer_memory(
        &self,
        memory: &Memory,
        offset: u64,
        buffer: &mut Buffer,
    ) -> Result<(), device::BindError> {
        debug!("usage={:?}, props={:b}", buffer.internal.usage, memory.properties);

        #[allow(non_snake_case)]
        let MiscFlags = if buffer.bind & (d3d11::D3D11_BIND_SHADER_RESOURCE |
                                                  d3d11::D3D11_BIND_UNORDERED_ACCESS) != 0 {
            d3d11::D3D11_RESOURCE_MISC_BUFFER_ALLOW_RAW_VIEWS
        } else {
            0
        };

        let initial_data = memory.host_visible.as_ref().map(|p| d3d11::D3D11_SUBRESOURCE_DATA {
            pSysMem: unsafe { p.borrow().as_ptr().offset(offset as isize) as _ },
            SysMemPitch: 0,
            SysMemSlicePitch: 0
        });

        let raw = match memory.ty {
            MemoryHeapFlags::DEVICE_LOCAL => {
                // device local memory
                let desc = d3d11::D3D11_BUFFER_DESC {
                    ByteWidth: buffer.requirements.size as _,
                    Usage: d3d11::D3D11_USAGE_DEFAULT,
                    BindFlags: buffer.requirements.size as _,
                    CPUAccessFlags: 0,
                    MiscFlags,
                    StructureByteStride: if buffer.internal.usage.contains(buffer::Usage::TRANSFER_SRC) { 4 } else { 0 },
                };

                let mut buffer: *mut d3d11::ID3D11Buffer = ptr::null_mut();
                let hr = unsafe {
                    self.raw.CreateBuffer(
                        &desc,
                        if let Some(data) = initial_data {
                            &data
                        } else {
                            ptr::null_mut()
                        },
                        &mut buffer as *mut *mut _ as *mut *mut _
                    )
                };

                if !winerror::SUCCEEDED(hr) {
                    return Err(device::BindError::WrongMemory);
                }

                unsafe { ComPtr::from_raw(buffer) }
            },
            MemoryHeapFlags::HOST_NONCOHERENT | MemoryHeapFlags::HOST_COHERENT => {
                let desc = d3d11::D3D11_BUFFER_DESC {
                    ByteWidth: buffer.requirements.size as _,
                    // TODO: dynamic?
                    Usage: d3d11::D3D11_USAGE_DEFAULT,
                    BindFlags: buffer.bind,
                    CPUAccessFlags: 0,
                    MiscFlags,
                    StructureByteStride: if buffer.internal.usage.contains(buffer::Usage::TRANSFER_SRC) { 4 } else { 0 },
                };

                let mut buffer: *mut d3d11::ID3D11Buffer = ptr::null_mut();
                let hr = unsafe {
                    self.raw.CreateBuffer(
                        &desc,
                        if let Some(data) = initial_data {
                            &data
                        } else {
                            ptr::null_mut()
                        },
                        &mut buffer as *mut *mut _ as *mut *mut _
                    )
                };

                if !winerror::SUCCEEDED(hr) {
                    return Err(device::BindError::WrongMemory);
                }

                unsafe { ComPtr::from_raw(buffer) }
            },
            _ => unimplemented!()
        };

        let disjoint_cb = if buffer.internal.disjoint_cb.is_some() {
            let desc = d3d11::D3D11_BUFFER_DESC {
                ByteWidth: buffer.requirements.size as _,
                Usage: d3d11::D3D11_USAGE_DEFAULT,
                BindFlags: d3d11::D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: 0,
                MiscFlags: 0,
                StructureByteStride: 0,
            };

            let mut buffer: *mut d3d11::ID3D11Buffer = ptr::null_mut();
            let hr = unsafe {
                self.raw.CreateBuffer(
                    &desc,
                    if let Some(data) = initial_data {
                        &data
                    } else {
                        ptr::null_mut()
                    },
                    &mut buffer as *mut *mut _ as *mut *mut _
                )
            };

            if !winerror::SUCCEEDED(hr) {
                return Err(device::BindError::WrongMemory);
            }

            Some(buffer)
        } else {
            None
        };

        let srv = if buffer.bind & d3d11::D3D11_BIND_SHADER_RESOURCE != 0 {
            let mut desc = unsafe { mem::zeroed::<d3d11::D3D11_SHADER_RESOURCE_VIEW_DESC>() };
            desc.Format = dxgiformat::DXGI_FORMAT_R32_TYPELESS;
            desc.ViewDimension = d3dcommon::D3D11_SRV_DIMENSION_BUFFEREX;
            unsafe {
                *desc.u.BufferEx_mut() = d3d11::D3D11_BUFFEREX_SRV {
                    FirstElement: 0,
                    // TODO: enforce alignment through HAL limits
                    NumElements: buffer.requirements.size as u32 / 4,
                    Flags: d3d11::D3D11_BUFFEREX_SRV_FLAG_RAW,
                };
            };

            let mut srv = ptr::null_mut();
            let hr = unsafe {
                self.raw.CreateShaderResourceView(
                    raw.as_raw() as *mut _,
                    &desc,
                    &mut srv as *mut *mut _ as *mut *mut _
                )
            };

            if !winerror::SUCCEEDED(hr) {
                error!("CreateShaderResourceView failed: 0x{:x}", hr);

                return Err(device::BindError::WrongMemory);
            }

            Some(srv)
        } else {
            None
        };

        let uav = if buffer.bind & d3d11::D3D11_BIND_UNORDERED_ACCESS != 0 {
            let mut desc = unsafe { mem::zeroed::<d3d11::D3D11_UNORDERED_ACCESS_VIEW_DESC>() };
            desc.Format = dxgiformat::DXGI_FORMAT_R32_TYPELESS;
            desc.ViewDimension = d3d11::D3D11_UAV_DIMENSION_BUFFER;
            unsafe {
                *desc.u.Buffer_mut() = d3d11::D3D11_BUFFER_UAV {
                    FirstElement: 0,
                    NumElements: buffer.requirements.size as u32 / 4,
                    Flags: d3d11::D3D11_BUFFER_UAV_FLAG_RAW
                };
            };

            let mut uav = ptr::null_mut();
            let hr = unsafe {
                self.raw.CreateUnorderedAccessView(
                    raw.as_raw() as *mut _,
                    &desc,
                    &mut uav as *mut *mut _ as *mut *mut _
                )
            };

            if !winerror::SUCCEEDED(hr) {
                error!("CreateUnorderedAccessView failed: 0x{:x}", hr);

                return Err(device::BindError::WrongMemory);
            }

            Some(uav)
        } else {
            None
        };

        let internal = InternalBuffer {
            raw: raw.into_raw(),
            disjoint_cb,
            srv,
            uav,
            usage: buffer.internal.usage,
        };
        let range = offset..buffer.requirements.size;

        memory.bind_buffer(range.clone(), internal.clone());

        let host_ptr = if let Some(vec) = &memory.host_visible {
            vec.borrow().as_ptr() as *mut _
        } else {
            ptr::null_mut()
        };

        buffer.internal = internal;
        buffer.ty = memory.ty;
        buffer.host_ptr = host_ptr;
        buffer.bound_range = range;

        Ok(())
    }
