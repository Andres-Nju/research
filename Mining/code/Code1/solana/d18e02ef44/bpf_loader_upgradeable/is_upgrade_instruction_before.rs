pub fn is_upgrade_instruction(instruction_data: &[u8]) -> bool {
    3 == instruction_data[0]
}
