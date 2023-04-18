    fn test_bad_convert() {
        // Test that error is returned when bad string is provided
        let p = Person::try_from("");
        assert!(p.is_err());
    }
