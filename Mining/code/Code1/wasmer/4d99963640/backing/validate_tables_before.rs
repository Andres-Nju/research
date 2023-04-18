    fn validate_tables(
        module: &ModuleInner,
        imports: &ImportBacking,
        tables: &mut SliceMap<LocalTableIndex, Table>,
    ) -> LinkResult<()> {
        for init in &module.info.elem_initializers {
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

            match init.table_index.local_or_import(&module.info) {
                LocalOrImport::Local(local_table_index) => {
                    let table = &tables[local_table_index];

                    if (table.size() as usize) < init_base + init.elements.len() {
                        return Err(vec![LinkError::Generic {
                            message: "elements segment does not fit".to_string(),
                        }]);
                    }
                }
                LocalOrImport::Import(import_table_index) => {
                    let table = &imports.tables[import_table_index];

                    if (table.size() as usize) < init_base + init.elements.len() {
                        return Err(vec![LinkError::Generic {
                            message: "elements segment does not fit".to_string(),
                        }]);
                    }
                }
            }
        }
        Ok(())
    }
