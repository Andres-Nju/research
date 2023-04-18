        fn list_style_should_show_all_properties_when_values_are_set() {
            let mut properties = Vec::new();

            let position = DeclaredValue::Value(ListStylePosition::inside);
            let image = DeclaredValue::Value(ListStyleImage::Url(
                Url::parse("http://servo/test.png").unwrap()
            ));
            let style_type = DeclaredValue::Value(ListStyleType::disc);

            properties.push(PropertyDeclaration::ListStylePosition(position));
            properties.push(PropertyDeclaration::ListStyleImage(image));
            properties.push(PropertyDeclaration::ListStyleType(style_type));

            let serialization = shorthand_properties_to_string(properties);
            assert_eq!(serialization, "list-style: inside url(\"http://servo/test.png\") disc;");
        }
