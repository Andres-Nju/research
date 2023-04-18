    fn finalize_memories(
        module: &ModuleInner,
        imports: &ImportBacking,
        memories: &mut SliceMap<LocalMemoryIndex, Memory>,
    ) -> BoxedMap<LocalMemoryIndex, *mut vm::LocalMemory> {
        // For each init that has some data...
        for init in module
            .info
            .data_initializers
            .iter()
            .filter(|init| init.data.len() > 0)
        {
            let init_base = match init.base {
                Initializer::Const(Value::I32(offset)) => offset as u32,
                Initializer::Const(_) => panic!("a const initializer must be the i32 type"),
                Initializer::GetGlobal(import_global_index) => {
                    if let Value::I32(x) = imports.globals[import_global_index].get() {
                        x as u32
                    } else {
                        panic!("unsupported global type for initialzer")
                    }
                }
            } as usize;

            match init.memory_index.local_or_import(&module.info) {
                LocalOrImport::Local(local_memory_index) => {
                    let memory_desc = module.info.memories[local_memory_index];
                    let data_top = init_base + init.data.len();
                    assert!(memory_desc.minimum.bytes().0 >= data_top);

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
                    unsafe {
                        let local_memory = &*imports.vm_memories[imported_memory_index];
                        let memory_slice =
                            slice::from_raw_parts_mut(local_memory.base, local_memory.bound);

                        let mem_init_view =
                            &mut memory_slice[init_base..init_base + init.data.len()];
                        mem_init_view.copy_from_slice(&init.data);
                    }
                }
            }
        }

        memories
            .iter_mut()
            .map(|(_, mem)| mem.vm_local_memory())
            .collect::<Map<_, _>>()
            .into_boxed_map()
    }
