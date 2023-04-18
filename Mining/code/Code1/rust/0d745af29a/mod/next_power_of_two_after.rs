        pub fn next_power_of_two(self) -> Self {
            // Call the trait to get overflow checks
            ops::Add::add(self.one_less_than_next_power_of_two(), 1)
        }
