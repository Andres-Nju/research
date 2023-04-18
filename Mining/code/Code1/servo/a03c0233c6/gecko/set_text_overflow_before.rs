    pub fn set_text_overflow(&mut self, v: longhands::text_overflow::computed_value::T) {
        use gecko_bindings::structs::nsStyleTextOverflowSide;
        use properties::longhands::text_overflow::{SpecifiedValue, Side};

        fn set(side: &mut nsStyleTextOverflowSide, value: &Side) {
            let ty = match *value {
                Side::Clip => structs::NS_STYLE_TEXT_OVERFLOW_CLIP,
                Side::Ellipsis => structs::NS_STYLE_TEXT_OVERFLOW_ELLIPSIS,
                Side::String(ref s) => {
                    side.mString.assign_utf8(s);
                    structs::NS_STYLE_TEXT_OVERFLOW_STRING
                }
            };
            side.mType = ty as u8;
        }

        self.clear_overflow_sides_if_string();
        if v.second.is_none() {
            self.gecko.mTextOverflow.mLogicalDirections = true;
        }

        let SpecifiedValue { ref first, ref second } = v;
        let second = second.as_ref().unwrap_or(&first);

        set(&mut self.gecko.mTextOverflow.mLeft, first);
        set(&mut self.gecko.mTextOverflow.mRight, second);
    }
