        fn to_css_declared<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
            // mako doesn't like ampersands following `<`
            fn extract_value<T>(x: &DeclaredValue<T>) -> Option< &T> {
                match *x {
                    DeclaredValue::Value(ref val) => Some(val),
                    _ => None,
                }
            }
            use std::cmp;
            let mut len = 0;
            % for name in "image mode position_x position_y size repeat origin clip composite".split():
                len = cmp::max(len, extract_value(self.mask_${name}).map(|i| i.0.len())
                                                                   .unwrap_or(0));
            % endfor

            // There should be at least one declared value
            if len == 0 {
                return dest.write_str("")
            }

            for i in 0..len {
                % for name in "image mode position_x position_y size repeat origin clip composite".split():
                    let ${name} = if let DeclaredValue::Value(ref arr) = *self.mask_${name} {
                        arr.0.get(i % arr.0.len())
                    } else {
                        None
                    };
                % endfor

                if let Some(image) = image {
                    try!(image.to_css(dest));
                } else {
                    try!(write!(dest, "none"));
                }

                try!(write!(dest, " "));

                if let Some(mode) = mode {
                    try!(mode.to_css(dest));
                } else {
                    try!(write!(dest, "match-source"));
                }

                try!(write!(dest, " "));

                try!(position_x.unwrap_or(&mask_position_x::single_value
                                                      ::get_initial_position_value())
                     .to_css(dest));

                try!(write!(dest, " "));

                try!(position_y.unwrap_or(&mask_position_y::single_value
                                                      ::get_initial_position_value())
                     .to_css(dest));

                if let Some(size) = size {
                    try!(write!(dest, " / "));
                    try!(size.to_css(dest));
                }

                try!(write!(dest, " "));

                if let Some(repeat) = repeat {
                    try!(repeat.to_css(dest));
                } else {
                    try!(write!(dest, "repeat"));
                }

                match (origin, clip) {
                    (Some(origin), Some(clip)) => {
                        use properties::longhands::mask_origin::single_value::computed_value::T as Origin;
                        use properties::longhands::mask_clip::single_value::computed_value::T as Clip;

                        try!(write!(dest, " "));

                        match (origin, clip) {
                            (&Origin::padding_box, &Clip::padding_box) => {
                                try!(origin.to_css(dest));
                            },
                            (&Origin::border_box, &Clip::border_box) => {
                                try!(origin.to_css(dest));
                            },
                            (&Origin::content_box, &Clip::content_box) => {
                                try!(origin.to_css(dest));
                            },
                            _ => {
                                try!(origin.to_css(dest));
                                try!(write!(dest, " "));
                                try!(clip.to_css(dest));
                            }
                        }
                    },
                    _ => {}
                };

                try!(write!(dest, " "));

                if let Some(composite) = composite {
                    try!(composite.to_css(dest));
                } else {
                    try!(write!(dest, "add"));
                }
            }

            Ok(())
        }
