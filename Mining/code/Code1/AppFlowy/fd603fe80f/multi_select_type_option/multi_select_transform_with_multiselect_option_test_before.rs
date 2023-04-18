    fn multi_select_transform_with_multiselect_option_test() {
        let mut singleselect_type_option_builder = SingleSelectTypeOptionBuilder::default();

        let google = SelectOptionPB::new("Google");
        singleselect_type_option_builder = singleselect_type_option_builder.add_option(google);

        let facebook = SelectOptionPB::new("Facebook");
        singleselect_type_option_builder = singleselect_type_option_builder.add_option(facebook);

        let singleselect_type_option_data = singleselect_type_option_builder.serializer().json_str();

        let mut multi_select = MultiSelectTypeOptionBuilder::default();
        multi_select.transform(&FieldType::MultiSelect, singleselect_type_option_data.clone());
        debug_assert_eq!(multi_select.0.options.len(), 2);

        // Already contain the yes/no option. It doesn't need to insert new options
        multi_select.transform(&FieldType::MultiSelect, singleselect_type_option_data);
        debug_assert_eq!(multi_select.0.options.len(), 2);
    }
