        fn from_computed_value(computed: &computed_value::T) -> Self {
            SpecifiedValue::Length(LengthOrPercentage::Length(
                ToComputedValue::from_computed_value(computed)
            ))
        }
