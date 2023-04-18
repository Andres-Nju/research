            fn query(name: &str) -> Option<$crate::pso::buffer::Element<$runtime_format>> {
                use std::mem::{size_of, transmute};
                use $crate::pso::buffer::{Element, ElemOffset};
                // using "1" here as a simple non-zero pointer addres
                let tmp: &$root = unsafe{ transmute(1usize) };
                let base = tmp as *const _ as usize;
                //HACK: special treatment of array queries
                let (sub_name, big_offset) = {
                    let mut split = name.split(|c| c == '[' || c == ']');
                    let _ = split.next().unwrap();
                    match split.next() {
                        Some(s) => {
                            let array_id: ElemOffset = s.parse().unwrap();
                            let sub_name = match split.next() {
                                Some(s) if s.starts_with('.') => &s[1..],
                                _ => name,
                            };
                            (sub_name, array_id * (size_of::<$root>() as ElemOffset))
                        },
                        None => (name, 0),
                    }
                };
                match sub_name {
                $(
                    $name => Some(Element {
                        format: <$ty as $compile_format>::get_format(),
                        offset: ((&tmp.$field as *const _ as usize) - base) as ElemOffset + big_offset,
                    }),
                )*
                    _ => None,
                }
            }
