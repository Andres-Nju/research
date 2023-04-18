    fn finalize_memories(
        module: &ModuleInner,
        imports: &ImportBacking,
        memories: &mut SliceMap<LocalMemoryIndex, Memory>,
    ) -> LinkResult<BoxedMap<LocalMemoryIndex, *mut vm::LocalMemory>> {
        // For each init that has some data...
        // Initialize data
        for init in module.info.data_initializers.iter() {
            let init_base = match init.base {
                Initializer::Const(Value::I32(offset)) => offset as u32,
                Initializer::Const(_) => {
                    return Err(vec![LinkError::Generic {
                        message: "a const initializer must be the i32 type".to_string(),
                    }]);
                }
                Initializer::GetGlobal(import_global_index) => {
                    if import_global_index.index() >= imports.globals.len() {
                        return Err(vec![LinkError::Generic {
                            message: "incorrect global index for initializer".to_string(),
                        }]);
                    }
                    if let Value::I32(x) = imports.globals[import_global_index].get() {
                        x as u32
                    } else {
                        return Err(vec![LinkError::Generic {
                            message: "unsupported global type for initializer".to_string(),
                        }]);
                    }
                }
            } as usize;

            match init.memory_index.local_or_import(&module.info) {
                LocalOrImport::Local(local_memory_index) => {
                    let mem = &memories[local_memory_index];
                    for (mem_byte, data_byte) in mem.view()[init_base..init_base + init.data.len()]
                        .iter()
                        .zip(init.data.iter())
                    {
                        mem_byte.set(*data_byte);
                    }
                }
                LocalOrImport::Import(imported_memory_index) => {
                    // Write the initialization data to the memory that
                    // we think the imported memory is.
                    let memory_slice = unsafe {
                        let local_memory = &*imports.vm_memories[imported_memory_index];
                        slice::from_raw_parts_mut(local_memory.base, local_memory.bound)
                    };

                    let mem_init_view = &mut memory_slice[init_base..init_base + init.data.len()];
                    mem_init_view.copy_from_slice(&init.data);
                }
            }
        }

        Ok(memories
            .iter_mut()
            .map(|(_, mem)| mem.vm_local_memory())
            .collect::<Map<_, _>>()
            .into_boxed_map())
    }
