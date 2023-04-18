pub fn is_set_authority_instruction(instruction_data: &[u8]) -> bool {
    4 == instruction_data[0]
}
