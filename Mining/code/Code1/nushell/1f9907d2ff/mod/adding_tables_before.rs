fn adding_tables() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            [[a b]; [1 2]] ++ [[4 5]; [10 11]] | to nuon
        "#
    ));
    assert_eq!(actual.out, "[{a: 1, b: 2}, {4: 10, 5: 11}]");
}
