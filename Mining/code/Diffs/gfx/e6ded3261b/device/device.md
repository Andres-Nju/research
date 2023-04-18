File_Code/gfx/e6ded3261b/device/device_after.rs --- Rust
   .                                                                                                                                                         2040         // Image usages which require RT/DS heap due to internal implementation.
   .                                                                                                                                                         2041         let target_usage = image::Usage::COLOR_ATTACHMENT | image::Usage::DEPTH_STENCIL_ATTACHMENT
   .                                                                                                                                                         2042             | image::Usage::TRANSFER_DST;
2039                                                                                                                                                         2043 
2040         let type_mask_shift = if self.private_caps.heterogeneous_resource_heaps {                                                                       2044         let type_mask_shift = if self.private_caps.heterogeneous_resource_heaps {
2041             MEM_TYPE_UNIVERSAL_SHIFT                                                                                                                    2045             MEM_TYPE_UNIVERSAL_SHIFT
2042         } else if usage.can_target() {                                                                                                                  2046         } else if usage.intersects(target_usage) {

