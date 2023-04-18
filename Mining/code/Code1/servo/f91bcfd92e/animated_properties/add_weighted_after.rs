    fn add_weighted(&self, other: &Self, self_portion: f64, other_portion: f64) -> Result<Self, ()> {
        let a = self.0 as f64;
        let b = other.0 as f64;
        const NORMAL: f64 = 400.;
        let weight = (a - NORMAL) * self_portion + (b - NORMAL) * other_portion + NORMAL;
        let weight = (weight.max(100.).min(900.) / 100.).round() * 100.;
        Ok(FontWeight(weight as u16))
    }
