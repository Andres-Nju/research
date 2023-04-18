    unsafe fn acquire_image(
        &mut self,
        mut timeout_ns: u64,
    ) -> Result<(Self::SwapchainImage, Option<w::Suboptimal>), w::AcquireError> {
        use hal::window::Swapchain as _;

        let ssc = self.swapchain.as_mut().unwrap();
        let moment = Instant::now();
        let (index, suboptimal) =
            ssc.swapchain
                .acquire_image(timeout_ns, None, Some(&ssc.fence))?;
        timeout_ns = timeout_ns.saturating_sub(moment.elapsed().as_nanos() as u64);
        let fences = &[ssc.fence.0];

        match ssc.device.0.wait_for_fences(fences, true, timeout_ns) {
            Ok(()) => {
                ssc.device.0.reset_fences(fences).unwrap();
                let frame = &ssc.frames[index as usize];
                // We have just waited for the frame to be fully available on CPU.
                // All the associated framebuffers are expected to be destroyed by now.
                for framebuffer in frame.framebuffers.0.lock().unwrap().framebuffers.drain() {
                    ssc.device.0.destroy_framebuffer(framebuffer, None);
                }
                let image = Self::SwapchainImage {
                    index,
                    view: native::ImageView {
                        image: frame.image,
                        view: frame.view,
                        range: hal::image::SubresourceRange {
                            aspects: hal::format::Aspects::COLOR,
                            layers: 0 .. 1,
                            levels: 0 .. 1,
                        },
                        owner: native::ImageViewOwner::Surface(FramebufferCachePtr(Arc::clone(
                            &frame.framebuffers.0,
                        ))),
                    },
                };
                Ok((image, suboptimal))
            }
            Err(vk::Result::NOT_READY) => Err(w::AcquireError::NotReady),
            Err(vk::Result::TIMEOUT) => Err(w::AcquireError::Timeout),
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Err(w::AcquireError::OutOfDate),
            Err(vk::Result::ERROR_SURFACE_LOST_KHR) => {
                Err(w::AcquireError::SurfaceLost(hal::device::SurfaceLost))
            }
            Err(vk::Result::ERROR_OUT_OF_HOST_MEMORY) => Err(w::AcquireError::OutOfMemory(
                hal::device::OutOfMemory::Host,
            )),
            Err(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY) => Err(w::AcquireError::OutOfMemory(
                hal::device::OutOfMemory::Device,
            )),
            Err(vk::Result::ERROR_DEVICE_LOST) => {
                Err(w::AcquireError::DeviceLost(hal::device::DeviceLost))
            }
            _ => unreachable!(),
        }
    }
