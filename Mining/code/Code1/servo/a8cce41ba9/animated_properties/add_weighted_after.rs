    fn add_weighted(&self, other: &Option<T>, self_portion: f64, other_portion: f64) -> Result<Option<T>, ()> {
        match (self, other) {
            (&Some(ref this), &Some(ref other)) => {
                Ok(this.add_weighted(other, self_portion, other_portion).ok())
            }
            (&None, &None) => Ok(None),
            _ => Err(()),
        }
    }
