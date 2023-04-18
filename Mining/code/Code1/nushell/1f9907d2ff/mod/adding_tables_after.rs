fn adding_tables() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            [[a b]; [1 2]] ++ [[c d]; [10 11]] | to nuon
        "#
    ));
    assert_eq!(actual.out, "[{a: 1, b: 2}, {c: 10, d: 11}]");
}
