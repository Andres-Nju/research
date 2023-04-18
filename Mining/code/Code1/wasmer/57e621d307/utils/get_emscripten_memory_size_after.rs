pub fn get_emscripten_memory_size(module: &Module) -> Result<(Pages, Option<Pages>, bool), String> {
    if module.info().imported_memories.len() == 0 {
        return Err("Emscripten requires at least one imported memory".to_string());
    }
    let (_, memory) = &module.info().imported_memories[ImportedMemoryIndex::new(0)];
    Ok((memory.minimum, memory.maximum, memory.shared))
}
