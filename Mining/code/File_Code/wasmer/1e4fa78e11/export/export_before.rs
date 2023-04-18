//! Create, read, destroy export definitions (function, global, memory
//! and table) on an instance.

use crate::{
    error::{update_last_error, CApiError},
    global::wasmer_global_t,
    import::wasmer_import_func_t,
    memory::wasmer_memory_t,
    module::wasmer_module_t,
    table::wasmer_table_t,
    value::{wasmer_value, wasmer_value_t, wasmer_value_tag},
    wasmer_byte_array, wasmer_result_t,
};
use libc::{c_int, c_uint};
use std::{ptr, slice};
use wasmer_runtime::{Instance, Memory, Module, Value};
use wasmer_runtime_core::{export::Export, module::ExportIndex};

/// Intermediate representation of an `Export` instance that is
/// exposed to C.
pub(crate) struct NamedExport {
    /// The export name.
    pub(crate) name: String,

    /// The export instance.
    pub(crate) export: Export,

    /// The instance that holds the export.
    pub(crate) instance: *mut Instance,
}

/// Opaque pointer to `NamedExport`.
#[repr(C)]
#[derive(Clone)]
pub struct wasmer_export_t;

/// Opaque pointer to `wasmer_export_t`.
#[repr(C)]
#[derive(Clone)]
pub struct wasmer_export_func_t;

/// Intermediate representation of a vector of `NamedExport` that is
/// exposed to C.
pub(crate) struct NamedExports(pub Vec<NamedExport>);

/// Opaque pointer to `NamedExports`.
#[repr(C)]
#[derive(Clone)]
pub struct wasmer_exports_t;

/// Intermediate representation of an export descriptor that is
/// exposed to C.
pub(crate) struct NamedExportDescriptor {
    /// The export name.
    name: String,

    /// The export kind.
    kind: wasmer_import_export_kind,
}

/// Opaque pointer to `NamedExportDescriptor`.
#[repr(C)]
#[derive(Clone)]
pub struct wasmer_export_descriptor_t;

/// Intermediate representation of a vector of `NamedExportDescriptor`
/// that is exposed to C.
pub struct NamedExportDescriptors(Vec<NamedExportDescriptor>);

/// Opaque pointer to `NamedExportDescriptors`.
#[repr(C)]
#[derive(Clone)]
pub struct wasmer_export_descriptors_t;

/// Union of import/export value.
#[repr(C)]
#[derive(Clone, Copy)]
pub union wasmer_import_export_value {
    pub func: *const wasmer_import_func_t,
    pub table: *const wasmer_table_t,
    pub memory: *const wasmer_memory_t,
    pub global: *const wasmer_global_t,
}

/// List of export/import kinds.
#[allow(non_camel_case_types)]
#[repr(u32)]
#[derive(Clone)]
pub enum wasmer_import_export_kind {
    WASM_FUNCTION,
    WASM_GLOBAL,
    WASM_MEMORY,
    WASM_TABLE,
}

/// Gets export descriptors for the given module
///
/// The caller owns the object and should call `wasmer_export_descriptors_destroy` to free it.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_export_descriptors(
    module: *const wasmer_module_t,
    export_descriptors: *mut *mut wasmer_export_descriptors_t,
) {
    let module = &*(module as *const Module);

    let named_export_descriptors: Box<NamedExportDescriptors> = Box::new(NamedExportDescriptors(
        module.info().exports.iter().map(|e| e.into()).collect(),
    ));
    *export_descriptors =
        Box::into_raw(named_export_descriptors) as *mut wasmer_export_descriptors_t;
}

/// Frees the memory for the given export descriptors
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn wasmer_export_descriptors_destroy(
    export_descriptors: *mut wasmer_export_descriptors_t,
) {
    if !export_descriptors.is_null() {
        unsafe { Box::from_raw(export_descriptors as *mut NamedExportDescriptors) };
    }
}

/// Gets the length of the export descriptors
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_export_descriptors_len(
    exports: *mut wasmer_export_descriptors_t,
) -> c_int {
    if exports.is_null() {
        return 0;
    }
    (*(exports as *mut NamedExportDescriptors)).0.len() as c_int
}

/// Gets export descriptor by index
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_export_descriptors_get(
    export_descriptors: *mut wasmer_export_descriptors_t,
    idx: c_int,
) -> *mut wasmer_export_descriptor_t {
    if export_descriptors.is_null() {
        return ptr::null_mut();
    }
    let named_export_descriptors = &mut *(export_descriptors as *mut NamedExportDescriptors);
    &mut (*named_export_descriptors).0[idx as usize] as *mut NamedExportDescriptor
        as *mut wasmer_export_descriptor_t
}

/// Gets name for the export descriptor
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_descriptor_name(
    export_descriptor: *mut wasmer_export_descriptor_t,
) -> wasmer_byte_array {
    let named_export_descriptor = &*(export_descriptor as *mut NamedExportDescriptor);
    wasmer_byte_array {
        bytes: named_export_descriptor.name.as_ptr(),
        bytes_len: named_export_descriptor.name.len() as u32,
    }
}

/// Gets export descriptor kind
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_descriptor_kind(
    export: *mut wasmer_export_descriptor_t,
) -> wasmer_import_export_kind {
    let named_export_descriptor = &*(export as *mut NamedExportDescriptor);
    named_export_descriptor.kind.clone()
}

/// Frees the memory for the given exports
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn wasmer_exports_destroy(exports: *mut wasmer_exports_t) {
    if !exports.is_null() {
        unsafe { Box::from_raw(exports as *mut NamedExports) };
    }
}

/// Gets the length of the exports
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_exports_len(exports: *mut wasmer_exports_t) -> c_int {
    if exports.is_null() {
        return 0;
    }
    (*(exports as *mut NamedExports)).0.len() as c_int
}

/// Gets wasmer_export by index
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_exports_get(
    exports: *mut wasmer_exports_t,
    idx: c_int,
) -> *mut wasmer_export_t {
    if exports.is_null() {
        return ptr::null_mut();
    }
    let named_exports = &mut *(exports as *mut NamedExports);
    &mut (*named_exports).0[idx as usize] as *mut NamedExport as *mut wasmer_export_t
}

/// Gets wasmer_export kind
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_kind(
    export: *mut wasmer_export_t,
) -> wasmer_import_export_kind {
    let named_export = &*(export as *mut NamedExport);
    match named_export.export {
        Export::Table(_) => wasmer_import_export_kind::WASM_TABLE,
        Export::Function { .. } => wasmer_import_export_kind::WASM_FUNCTION,
        Export::Global(_) => wasmer_import_export_kind::WASM_GLOBAL,
        Export::Memory(_) => wasmer_import_export_kind::WASM_MEMORY,
    }
}

/// Sets the result parameter to the arity of the params of the wasmer_export_func_t
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_func_params_arity(
    func: *const wasmer_export_func_t,
    result: *mut u32,
) -> wasmer_result_t {
    let named_export = &*(func as *const NamedExport);
    let export = &named_export.export;
    if let Export::Function { ref signature, .. } = *export {
        *result = signature.params().len() as u32;
        wasmer_result_t::WASMER_OK
    } else {
        update_last_error(CApiError {
            msg: "func ptr error in wasmer_export_func_params_arity".to_string(),
        });
        wasmer_result_t::WASMER_ERROR
    }
}

/// Sets the params buffer to the parameter types of the given wasmer_export_func_t
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_func_params(
    func: *const wasmer_export_func_t,
    params: *mut wasmer_value_tag,
    params_len: u32,
) -> wasmer_result_t {
    let named_export = &*(func as *const NamedExport);
    let export = &named_export.export;
    if let Export::Function { ref signature, .. } = *export {
        let params: &mut [wasmer_value_tag] =
            slice::from_raw_parts_mut(params, params_len as usize);
        for (i, item) in signature.params().iter().enumerate() {
            params[i] = item.into();
        }
        wasmer_result_t::WASMER_OK
    } else {
        update_last_error(CApiError {
            msg: "func ptr error in wasmer_export_func_params".to_string(),
        });
        wasmer_result_t::WASMER_ERROR
    }
}

/// Sets the returns buffer to the parameter types of the given wasmer_export_func_t
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_func_returns(
    func: *const wasmer_export_func_t,
    returns: *mut wasmer_value_tag,
    returns_len: u32,
) -> wasmer_result_t {
    let named_export = &*(func as *const NamedExport);
    let export = &named_export.export;
    if let Export::Function { ref signature, .. } = *export {
        let returns: &mut [wasmer_value_tag] =
            slice::from_raw_parts_mut(returns, returns_len as usize);
        for (i, item) in signature.returns().iter().enumerate() {
            returns[i] = item.into();
        }
        wasmer_result_t::WASMER_OK
    } else {
        update_last_error(CApiError {
            msg: "func ptr error in wasmer_export_func_returns".to_string(),
        });
        wasmer_result_t::WASMER_ERROR
    }
}

/// Sets the result parameter to the arity of the returns of the wasmer_export_func_t
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_func_returns_arity(
    func: *const wasmer_export_func_t,
    result: *mut u32,
) -> wasmer_result_t {
    let named_export = &*(func as *const NamedExport);
    let export = &named_export.export;
    if let Export::Function { ref signature, .. } = *export {
        *result = signature.returns().len() as u32;
        wasmer_result_t::WASMER_OK
    } else {
        update_last_error(CApiError {
            msg: "func ptr error in wasmer_export_func_results_arity".to_string(),
        });
        wasmer_result_t::WASMER_ERROR
    }
}

/// Gets export func from export
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_to_func(
    export: *const wasmer_export_t,
) -> *const wasmer_export_func_t {
    export as *const wasmer_export_func_t
}

/// Gets a memory pointer from an export pointer.
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_to_memory(
    export: *const wasmer_export_t,
    memory: *mut *mut wasmer_memory_t,
) -> wasmer_result_t {
    let named_export = &*(export as *const NamedExport);
    let export = &named_export.export;

    if let Export::Memory(exported_memory) = export {
        *memory = exported_memory as *const Memory as *mut wasmer_memory_t;
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

/// Gets name from wasmer_export
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_name(export: *mut wasmer_export_t) -> wasmer_byte_array {
    let named_export = &*(export as *mut NamedExport);
    wasmer_byte_array {
        bytes: named_export.name.as_ptr(),
        bytes_len: named_export.name.len() as u32,
    }
}

/// Calls a `func` with the provided parameters.
/// Results are set using the provided `results` pointer.
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_export_func_call(
    func: *const wasmer_export_func_t,
    params: *const wasmer_value_t,
    params_len: c_uint,
    results: *mut wasmer_value_t,
    results_len: c_uint,
) -> wasmer_result_t {
    if func.is_null() {
        update_last_error(CApiError {
            msg: "func ptr is null".to_string(),
        });
        return wasmer_result_t::WASMER_ERROR;
    }

    if params_len > 0 && params.is_null() {
        update_last_error(CApiError {
            msg: "params ptr is null".to_string(),
        });
        return wasmer_result_t::WASMER_ERROR;
    }

    let params: Vec<Value> = {
        if params_len <= 0 {
            vec![]
        } else {
            slice::from_raw_parts::<wasmer_value_t>(params, params_len as usize)
                .iter()
                .cloned()
                .map(|x| x.into())
                .collect()
        }
    };

    let named_export = &*(func as *mut NamedExport);

    let results: &mut [wasmer_value_t] = slice::from_raw_parts_mut(results, results_len as usize);

    let instance = &*named_export.instance;
    let result = instance.call(&named_export.name, &params[..]);
    match result {
        Ok(results_vec) => {
            if !results_vec.is_empty() {
                let ret = match results_vec[0] {
                    Value::I32(x) => wasmer_value_t {
                        tag: wasmer_value_tag::WASM_I32,
                        value: wasmer_value { I32: x },
                    },
                    Value::I64(x) => wasmer_value_t {
                        tag: wasmer_value_tag::WASM_I64,
                        value: wasmer_value { I64: x },
                    },
                    Value::F32(x) => wasmer_value_t {
                        tag: wasmer_value_tag::WASM_F32,
                        value: wasmer_value { F32: x },
                    },
                    Value::F64(x) => wasmer_value_t {
                        tag: wasmer_value_tag::WASM_F64,
                        value: wasmer_value { F64: x },
                    },
                    Value::V128(_) => unimplemented!("returning V128 type"),
                };
                results[0] = ret;
            }
            wasmer_result_t::WASMER_OK
        }
        Err(err) => {
            update_last_error(err);
            wasmer_result_t::WASMER_ERROR
        }
    }
}

impl From<(&std::string::String, &ExportIndex)> for NamedExportDescriptor {
    fn from((name, export_index): (&String, &ExportIndex)) -> Self {
        let kind = match *export_index {
            ExportIndex::Memory(_) => wasmer_import_export_kind::WASM_MEMORY,
            ExportIndex::Global(_) => wasmer_import_export_kind::WASM_GLOBAL,
            ExportIndex::Table(_) => wasmer_import_export_kind::WASM_TABLE,
            ExportIndex::Func(_) => wasmer_import_export_kind::WASM_FUNCTION,
        };
        NamedExportDescriptor {
            name: name.clone(),
            kind,
        }
    }
}
