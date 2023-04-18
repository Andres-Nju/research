pub unsafe extern "C" fn wasmer_export_to_memory(
    export: *const wasmer_export_t,
    memory: *mut *mut wasmer_memory_t,
) -> wasmer_result_t {
    let named_export = &*(export as *const NamedExport);
    let export = &named_export.export;

    if let Export::Memory(exported_memory) = export {
        let mem = Box::new(exported_memory.clone());
        *memory = Box::into_raw(mem) as *mut wasmer_memory_t;
        wasmer_result_t::WASMER_OK
    } else {
        update_last_error(CApiError {
            msg: "cannot cast the `wasmer_export_t` pointer to a  `wasmer_memory_t` \
                  pointer because it does not represent a memory export."
                .to_string(),
        });
        wasmer_result_t::WASMER_ERROR
    }
}
