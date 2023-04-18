    fn create_fence(&self, _signaled: bool) -> n::Fence {
        let mut handle = ptr::null_mut();
        assert_eq!(winerror::S_OK, unsafe {
            self.raw.clone().CreateFence(
                0,
                d3d12::D3D12_FENCE_FLAG_NONE,
                &d3d12::IID_ID3D12Fence,
                &mut handle,
            )
        });

        n::Fence {
            raw: unsafe { ComPtr::new(handle as *mut _) },
        }
    }
