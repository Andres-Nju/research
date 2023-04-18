    fn validate_memories(module: &ModuleInner, imports: &ImportBacking) -> LinkResult<()> {
        // Validate data size fits
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

            // Validate data size fits
            match init.memory_index.local_or_import(&module.info) {
                LocalOrImport::Local(local_memory_index) => {
                    let memory_desc = module.info.memories[local_memory_index];
                    let data_top = init_base + init.data.len();
                    if memory_desc.minimum.bytes().0 < data_top || data_top < init_base {
                        return Err(vec![LinkError::Generic {
                            message: "data segment does not fit".to_string(),
                        }]);
                    }
                }
                LocalOrImport::Import(imported_memory_index) => {
                    // Write the initialization data to the memory that
                    // we think the imported memory is.
                    let local_memory = unsafe { &*imports.vm_memories[imported_memory_index] };
                    let data_top = init_base + init.data.len();
                    if local_memory.bound < data_top || data_top < init_base {
                        return Err(vec![LinkError::Generic {
                            message: "data segment does not fit".to_string(),
                        }]);
                    }
                }
            }
        }
        Ok(())
    }
