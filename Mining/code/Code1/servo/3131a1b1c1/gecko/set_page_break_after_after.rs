    pub fn set_page_break_after(&mut self, v: longhands::page_break_after::computed_value::T) {
        use computed_values::page_break_after::T;
        let result = match v {
            T::auto   => false,
            T::always => true,
            T::avoid  => false,
            T::left   => true,
            T::right  => true
        };
        self.gecko.mBreakAfter = result;
    }
