fn extract_bytecode_format_version(bc: &[u8]) -> u32 {
    let pos = link::RLIB_BYTECODE_OBJECT_VERSION_OFFSET;
    let byte_data = &bc[pos..pos + 4];
    let data = unsafe { read_unaligned(byte_data.as_ptr() as *const u32) };
    u32::from_le(data)
}
