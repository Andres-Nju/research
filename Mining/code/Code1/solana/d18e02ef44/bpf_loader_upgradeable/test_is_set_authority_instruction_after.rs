    fn test_is_set_authority_instruction() {
        assert!(!is_set_authority_instruction(&[]));
        assert_is_instruction(
            is_set_authority_instruction,
            UpgradeableLoaderInstruction::SetAuthority {},
        );
    }
