    fn test_is_upgrade_instruction() {
        assert!(!is_upgrade_instruction(&[]));
        assert_is_instruction(
            is_upgrade_instruction,
            UpgradeableLoaderInstruction::Upgrade {},
        );
    }
