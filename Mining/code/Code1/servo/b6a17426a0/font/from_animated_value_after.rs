    fn from_animated_value(animated: Self::AnimatedValue) -> Self {
        FontStretch(NonNegativePercentage::from_animated_value(animated))
    }
