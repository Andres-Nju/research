pub fn map_buffer_resource_state(access: buffer::Access) -> D3D12_RESOURCE_STATES {
    use self::buffer::Access;
    // Mutable states
    if access.contains(Access::SHADER_WRITE) {
        return D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
    }
    if access.contains(Access::TRANSFER_WRITE) {
        // Resolve not relevant for buffers.
        return D3D12_RESOURCE_STATE_COPY_DEST;
    }

    // Read-only states
    let mut state = D3D12_RESOURCE_STATE_COMMON;

    if access.contains(Access::TRANSFER_READ) {
        state |= D3D12_RESOURCE_STATE_COPY_SOURCE;
    }
    if access.contains(Access::INDEX_BUFFER_READ) {
        state |= D3D12_RESOURCE_STATE_INDEX_BUFFER;
    }
    if access.contains(Access::VERTEX_BUFFER_READ) || access.contains(Access::UNIFORM_READ)
    {
        state |= D3D12_RESOURCE_STATE_VERTEX_AND_CONSTANT_BUFFER;
    }
    if access.contains(Access::INDIRECT_COMMAND_READ) {
        state |= D3D12_RESOURCE_STATE_INDIRECT_ARGUMENT;
    }
    if access.contains(Access::SHADER_READ) {
        // SHADER_READ only allows SRV access
        state |= D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE
            | D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE;
    }

    state
}
