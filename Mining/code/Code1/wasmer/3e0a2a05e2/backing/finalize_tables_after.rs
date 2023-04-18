    fn finalize_tables(
        module: &ModuleInner,
        imports: &ImportBacking,
        tables: &mut SliceMap<LocalTableIndex, Table>,
        vmctx: *mut vm::Ctx,
    ) -> BoxedMap<LocalTableIndex, *mut vm::LocalTable> {
        for init in &module.info.elem_initializers {
            let init_base = match init.base {
                Initializer::Const(Value::I32(offset)) => offset as u32,
                Initializer::Const(_) => panic!("a const initializer must be the i32 type"),
                Initializer::GetGlobal(import_global_index) => {
                    if let Value::I32(x) = imports.globals[import_global_index].get() {
                        x as u32
                    } else {
                        panic!("unsupported global type for initializer")
                    }
                }
            } as usize;

            match init.table_index.local_or_import(&module.info) {
                LocalOrImport::Local(local_table_index) => {
                    let table = &tables[local_table_index];

                    if (table.size() as usize) < init_base + init.elements.len() {
                        let delta = (init_base + init.elements.len()) - table.size() as usize;
                        // Grow the table if it's too small.
                        table.grow(delta as u32).expect("couldn't grow table");
                    }

                    table.anyfunc_direct_access_mut(|elements| {
                        for (i, &func_index) in init.elements.iter().enumerate() {
                            let sig_index = module.info.func_assoc[func_index];
                            // let signature = &module.info.signatures[sig_index];
                            let signature = SigRegistry
                                .lookup_signature_ref(&module.info.signatures[sig_index]);
                            let sig_id =
                                vm::SigId(SigRegistry.lookup_sig_index(signature).index() as u32);

                            let (func, ctx) = match func_index.local_or_import(&module.info) {
                                LocalOrImport::Local(local_func_index) => (
                                    module
                                        .runnable_module
                                        .get_func(&module.info, local_func_index)
                                        .unwrap()
                                        .as_ptr()
                                        as *const vm::Func,
                                    vmctx,
                                ),
                                LocalOrImport::Import(imported_func_index) => {
                                    let vm::ImportedFunc { func, vmctx } =
                                        imports.vm_functions[imported_func_index];
                                    (func, vmctx)
                                }
                            };

                            elements[init_base + i] = vm::Anyfunc { func, ctx, sig_id };
                        }
                    });
                }
                LocalOrImport::Import(import_table_index) => {
                    let table = &imports.tables[import_table_index];

                    if (table.size() as usize) < init_base + init.elements.len() {
                        let delta = (init_base + init.elements.len()) - table.size() as usize;
                        // Grow the table if it's too small.
                        table.grow(delta as u32).expect("couldn't grow table");
                    }

                    table.anyfunc_direct_access_mut(|elements| {
                        for (i, &func_index) in init.elements.iter().enumerate() {
                            let sig_index = module.info.func_assoc[func_index];
                            let signature = SigRegistry
                                .lookup_signature_ref(&module.info.signatures[sig_index]);
                            // let signature = &module.info.signatures[sig_index];
                            let sig_id =
                                vm::SigId(SigRegistry.lookup_sig_index(signature).index() as u32);

                            let (func, ctx) = match func_index.local_or_import(&module.info) {
                                LocalOrImport::Local(local_func_index) => (
                                    module
                                        .runnable_module
                                        .get_func(&module.info, local_func_index)
                                        .unwrap()
                                        .as_ptr()
                                        as *const vm::Func,
                                    vmctx,
                                ),
                                LocalOrImport::Import(imported_func_index) => {
                                    let vm::ImportedFunc { func, vmctx } =
                                        imports.vm_functions[imported_func_index];
                                    (func, vmctx)
                                }
                            };

                            elements[init_base + i] = vm::Anyfunc { func, ctx, sig_id };
                        }
                    });
                }
            }
        }

        tables
            .iter_mut()
            .map(|(_, table)| table.vm_local_table())
            .collect::<Map<_, _>>()
            .into_boxed_map()
    }
