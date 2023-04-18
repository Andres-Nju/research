File_Code/nushell/c1bec3b443/calc/calc_after.rs --- Rust
 .                                                                                                                                                           52     use std::f64;
52     let num = meval::eval_str(math_expression);                                                                                                           53     let num = meval::eval_str(math_expression);
53     match num {                                                                                                                                           54     match num {
54         Ok(num) => Ok(UntaggedValue::from(Primitive::from(num)).into_value(tag)),                                                                         55         Ok(num) => {
                                                                                                                                                             56             if num == f64::INFINITY || num == f64::NEG_INFINITY {
                                                                                                                                                             57                 return Err(String::from("cannot represent result"));
                                                                                                                                                             58             }
                                                                                                                                                             59             Ok(UntaggedValue::from(Primitive::from(num)).into_value(tag))
                                                                                                                                                             60         }

