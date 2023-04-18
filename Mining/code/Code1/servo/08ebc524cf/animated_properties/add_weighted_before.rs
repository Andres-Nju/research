    fn add_weighted(&self, other: &i32, self_portion: f64, other_portion: f64) -> Result<Self, ()> {
        Ok((*self as f64 * self_portion + *other as f64 * other_portion).round() as i32)
    }
