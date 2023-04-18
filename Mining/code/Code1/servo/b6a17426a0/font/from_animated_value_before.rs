    fn from_animated_value(animated: Self::AnimatedValue) -> Self {
        FontStretch(NonNegative(animated))
    }
