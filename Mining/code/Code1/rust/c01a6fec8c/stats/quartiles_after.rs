    fn quartiles(&self) -> (f64, f64, f64) {
        let mut tmp = self.to_vec();
        local_sort(&mut tmp);
        let first = 25f64;
        let a = percentile_of_sorted(&tmp, first);
        let second = 50f64;
        let b = percentile_of_sorted(&tmp, second);
        let third = 75f64;
        let c = percentile_of_sorted(&tmp, third);
        (a, b, c)
    }
