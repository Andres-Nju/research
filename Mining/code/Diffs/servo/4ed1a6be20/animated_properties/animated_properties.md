File_Code/servo/4ed1a6be20/animated_properties/animated_properties_after.rs --- Text (1510 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1343             TransformOperation::Rotate(..) => {                                                                                                         1343             TransformOperation::Rotate(x, y, z, a) => {
1344                 result.push(TransformOperation::Rotate(0.0, 0.0, 1.0, Angle::zero()));                                                                  1344                 let (x, y, z, _) = get_normalized_vector_and_angle(x, y, z, a);
                                                                                                                                                             1345                 result.push(TransformOperation::Rotate(x, y, z, Angle::zero()));

