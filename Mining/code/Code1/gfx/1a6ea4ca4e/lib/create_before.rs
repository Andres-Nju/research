    pub fn create(_: &str, _: u32) -> Instance {
        #[cfg(debug_assertions)]
        {
            // Enable debug layer
            let mut debug_controller: *mut d3d12sdklayers::ID3D12Debug = ptr::null_mut();
            let hr = unsafe {
                d3d12::D3D12GetDebugInterface(
                    &d3d12sdklayers::IID_ID3D12Debug,
                    &mut debug_controller as *mut *mut _ as *mut *mut _)
            };

            if winerror::SUCCEEDED(hr) {
                unsafe { (*debug_controller).EnableDebugLayer() };
            }

            unsafe { (*debug_controller).Release(); }
        }

        // Create DXGI factory
        let mut dxgi_factory: *mut dxgi1_4::IDXGIFactory4 = ptr::null_mut();

        let hr = unsafe {
            dxgi1_3::CreateDXGIFactory2(
                dxgi1_3::DXGI_CREATE_FACTORY_DEBUG,
                &dxgi1_4::IID_IDXGIFactory4,
                &mut dxgi_factory as *mut *mut _ as *mut *mut _)
        };

        if !winerror::SUCCEEDED(hr) {
            error!("Failed on dxgi factory creation: {:?}", hr);
        }

        Instance {
            factory: unsafe { ComPtr::from_raw(dxgi_factory) },
        }
    }
