    fn kind(&self) -> image::Kind;

    /// Check if the queue family supports presentation to this surface.
    ///
    /// # Examples
    ///
    /// ```no_run
    ///
    /// ```
    fn supports_queue_family(&self, family: &B::QueueFamily) -> bool;

    /// Query surface capabilities, formats, and present modes for this physical device.
    ///
    /// Use this function for configuring swapchain creation.
    ///
    /// Returns a tuple of surface capabilities and formats.
    /// If formats is `None` than the surface has no preferred format and the
    /// application may use any desired format.
    fn compatibility(
        &self,
        physical_device: &B::PhysicalDevice,
    ) -> (SurfaceCapabilities, Option<Vec<Format>>, Vec<PresentMode>);
}

/// Index of an image in the swapchain.
///
/// The swapchain is a series of one or more images, usually
/// with one being drawn on while the other is displayed by
/// the GPU (aka double-buffering). A `SwapImageIndex` refers
/// to a particular image in the swapchain.
pub type SwapImageIndex = u32;

/// Specifies the mode regulating how a swapchain presents frames.
#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum PresentMode {
    /// Don't ever wait for v-sync.
    Immediate = 0,
    /// Wait for v-sync, overwrite the last rendered frame.
    Mailbox = 1,
    /// Present frames in the same order they are rendered.
    Fifo = 2,
    /// Don't wait for the next v-sync if we just missed it.
    Relaxed = 3,
}
