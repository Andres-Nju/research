  fn test_parse_replacement_variables() {
    let actual = parse_replacement_variables(
      "https://deno.land/_vsc1/modules/${module}/v/${{version}}",
    );
    assert_eq!(actual.iter().count(), 2);
    assert!(actual.contains(&"module".to_owned()));
    assert!(actual.contains(&"version".to_owned()));
  }
