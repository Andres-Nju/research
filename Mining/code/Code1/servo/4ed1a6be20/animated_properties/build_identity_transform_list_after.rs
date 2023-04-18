fn build_identity_transform_list(list: &[TransformOperation]) -> Vec<TransformOperation> {
    let mut result = vec!();

    for operation in list {
        match *operation {
            TransformOperation::Matrix(..) => {
                let identity = ComputedMatrix::identity();
                result.push(TransformOperation::Matrix(identity));
            }
            TransformOperation::MatrixWithPercents(..) => {}
            TransformOperation::Skew(..) => {
                result.push(TransformOperation::Skew(Angle::zero(), Angle::zero()))
            }
            TransformOperation::Translate(..) => {
                result.push(TransformOperation::Translate(LengthOrPercentage::zero(),
                                                          LengthOrPercentage::zero(),
                                                          Au(0)));
            }
            TransformOperation::Scale(..) => {
                result.push(TransformOperation::Scale(1.0, 1.0, 1.0));
            }
            TransformOperation::Rotate(x, y, z, a) => {
                let (x, y, z, _) = get_normalized_vector_and_angle(x, y, z, a);
                result.push(TransformOperation::Rotate(x, y, z, Angle::zero()));
            }
            TransformOperation::Perspective(..) |
            TransformOperation::AccumulateMatrix { .. } |
            TransformOperation::InterpolateMatrix { .. } => {
                // Perspective: We convert a perspective function into an equivalent
                //     ComputedMatrix, and then decompose/interpolate/recompose these matrices.
                // AccumulateMatrix/InterpolateMatrix: We do interpolation on
                //     AccumulateMatrix/InterpolateMatrix by reading it as a ComputedMatrix
                //     (with layout information), and then do matrix interpolation.
                //
                // Therefore, we use an identity matrix to represent the identity transform list.
                // http://dev.w3.org/csswg/css-transforms/#identity-transform-function
                let identity = ComputedMatrix::identity();
                result.push(TransformOperation::Matrix(identity));
            }
        }
    }

    result
}
