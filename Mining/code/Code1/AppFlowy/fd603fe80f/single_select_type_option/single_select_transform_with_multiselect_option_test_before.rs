    fn single_select_transform_with_multiselect_option_test() {
        let mut multiselect_type_option_builder = MultiSelectTypeOptionBuilder::default();

        let google = SelectOptionPB::new("Google");
        multiselect_type_option_builder = multiselect_type_option_builder.add_option(google);

        let facebook = SelectOptionPB::new("Facebook");
        multiselect_type_option_builder = multiselect_type_option_builder.add_option(facebook);

        let multiselect_type_option_data = multiselect_type_option_builder.serializer().json_str();

        let mut single_select = SingleSelectTypeOptionBuilder::default();
        single_select.transform(&FieldType::MultiSelect, multiselect_type_option_data.clone());
        debug_assert_eq!(single_select.0.options.len(), 2);

        // Already contain the yes/no option. It doesn't need to insert new options
        single_select.transform(&FieldType::MultiSelect, multiselect_type_option_data);
        debug_assert_eq!(single_select.0.options.len(), 2);
    }
