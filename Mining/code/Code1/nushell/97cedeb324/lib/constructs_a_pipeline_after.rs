    fn constructs_a_pipeline() {
        let actual = pipeline(
            r#"
                open los_tres_amigos.txt
                | from-csv
                | get rusty_luck
                | str to-int
                | math sum
                | echo "$it"
            "#,
        );

        assert_eq!(
            actual,
            r#"open los_tres_amigos.txt | from-csv | get rusty_luck | str to-int | math sum | echo "$it""#
        );
    }
