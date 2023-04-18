    pub fn set_clip(&mut self, v: longhands::clip::computed_value::T) {
        use gecko_bindings::structs::NS_STYLE_CLIP_AUTO;
        use gecko_bindings::structs::NS_STYLE_CLIP_RECT;
        use gecko_bindings::structs::NS_STYLE_CLIP_LEFT_AUTO;
        use gecko_bindings::structs::NS_STYLE_CLIP_TOP_AUTO;
        use gecko_bindings::structs::NS_STYLE_CLIP_RIGHT_AUTO;
        use gecko_bindings::structs::NS_STYLE_CLIP_BOTTOM_AUTO;
        use values::Either;

        match v {
            Either::First(rect) => {
                self.gecko.mClipFlags = NS_STYLE_CLIP_RECT as u8;
                if let Some(left) = rect.left {
                    self.gecko.mClip.x = left.0;
                } else {
                    self.gecko.mClip.x = 0;
                    self.gecko.mClipFlags |= NS_STYLE_CLIP_LEFT_AUTO as u8;
                }

                if let Some(top) = rect.top {
                    self.gecko.mClip.y = top.0;
                } else {
                    self.gecko.mClip.y = 0;
                    self.gecko.mClipFlags |= NS_STYLE_CLIP_TOP_AUTO as u8;
                }

                if let Some(bottom) = rect.bottom {
                    self.gecko.mClip.height = bottom.0 - self.gecko.mClip.y;
                } else {
                    self.gecko.mClip.height = 1 << 30; // NS_MAXSIZE
                    self.gecko.mClipFlags |= NS_STYLE_CLIP_BOTTOM_AUTO as u8;
                }

                if let Some(right) = rect.right {
                    self.gecko.mClip.width = right.0 - self.gecko.mClip.x;
                } else {
                    self.gecko.mClip.width = 1 << 30; // NS_MAXSIZE
                    self.gecko.mClipFlags |= NS_STYLE_CLIP_RIGHT_AUTO as u8;
                }
            },
            Either::Second(_auto) => {
                self.gecko.mClipFlags = NS_STYLE_CLIP_AUTO as u8;
                self.gecko.mClip.x = 0;
                self.gecko.mClip.y = 0;
                self.gecko.mClip.width = 0;
                self.gecko.mClip.height = 0;
            }
        }
    }
