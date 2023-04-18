fn extract_compressed_bytecode_size_v1(bc: &[u8]) -> u64 {
    let pos = link::RLIB_BYTECODE_OBJECT_V1_DATASIZE_OFFSET;
    let byte_data = &bc[pos..pos + 8];
    let data = unsafe { read_unaligned(byte_data.as_ptr() as *const u64) };
    u64::from_le(data)
}
