File_Code/servo/08ebc524cf/animated_properties/animated_properties_after.rs --- Text (1476 errors, exceeded DFT_PARSE_ERROR_LIMIT)
866         Ok((*self as f64 * self_portion + *other as f64 * other_portion).round() as i32)                                                                 866         Ok((*self as f64 * self_portion + *other as f64 * other_portion + 0.5).floor() as i32)

