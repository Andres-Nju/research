    fn compute_distance(&self, other: &Self) -> Result<f64, ()> {
        match (self, other) {
            (&Some(ref this), &Some(ref other)) => {
                this.compute_distance(other)
            },
            (&None, &None) => Ok(0.0),
            _ => Err(()),
        }
    }
