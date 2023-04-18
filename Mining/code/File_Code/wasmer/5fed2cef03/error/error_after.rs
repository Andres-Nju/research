use crate::types::{FuncSig, GlobalDescriptor, MemoryDescriptor, TableDescriptor, Type};
use core::borrow::Borrow;
use std::any::Any;

pub type Result<T> = std::result::Result<T, Error>;
pub type CompileResult<T> = std::result::Result<T, CompileError>;
pub type LinkResult<T> = std::result::Result<T, Vec<LinkError>>;
pub type RuntimeResult<T> = std::result::Result<T, RuntimeError>;
pub type CallResult<T> = std::result::Result<T, CallError>;
pub type ResolveResult<T> = std::result::Result<T, ResolveError>;
pub type ParseResult<T> = std::result::Result<T, ParseError>;

/// This is returned when the chosen compiler is unable to
/// successfully compile the provided WebAssembly module into
/// a `Module`.
///
/// Comparing two `CompileError`s always evaluates to false.
#[derive(Debug, Clone)]
pub enum CompileError {
    ValidationError { msg: String },
    InternalError { msg: String },
}

impl PartialEq for CompileError {
    fn eq(&self, _other: &CompileError) -> bool {
        false
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CompileError::InternalError { msg } => {
                write!(f, "Internal compiler error: \"{}\"", msg)
            }
            CompileError::ValidationError { msg } => write!(f, "Validation error \"{}\"", msg),
        }
    }
}

impl std::error::Error for CompileError {}

/// This is returned when the runtime is unable to
/// correctly link the module with the provided imports.
///
/// Comparing two `LinkError`s always evaluates to false.
#[derive(Debug, Clone)]
pub enum LinkError {
    IncorrectImportType {
        namespace: String,
        name: String,
        expected: String,
        found: String,
    },
    IncorrectImportSignature {
        namespace: String,
        name: String,
        expected: FuncSig,
        found: FuncSig,
    },
    ImportNotFound {
        namespace: String,
        name: String,
    },
    IncorrectMemoryDescriptor {
        namespace: String,
        name: String,
        expected: MemoryDescriptor,
        found: MemoryDescriptor,
    },
    IncorrectTableDescriptor {
        namespace: String,
        name: String,
        expected: TableDescriptor,
        found: TableDescriptor,
    },
    IncorrectGlobalDescriptor {
        namespace: String,
        name: String,
        expected: GlobalDescriptor,
        found: GlobalDescriptor,
    },
}

impl PartialEq for LinkError {
    fn eq(&self, _other: &LinkError) -> bool {
        false
    }
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LinkError::ImportNotFound {namespace, name} => write!(f, "Import not found, namespace: {}, name: {}", namespace, name),
            LinkError::IncorrectGlobalDescriptor {namespace, name,expected,found} => {
                write!(f, "Incorrect global descriptor, namespace: {}, name: {}, expected global descriptor: {:?}, found global descriptor: {:?}", namespace, name, expected, found)
            },
            LinkError::IncorrectImportSignature{namespace, name,expected,found} => {
                write!(f, "Incorrect import signature, namespace: {}, name: {}, expected signature: {}, found signature: {}", namespace, name, expected, found)
            }
            LinkError::IncorrectImportType{namespace, name,expected,found} => {
                write!(f, "Incorrect import type, namespace: {}, name: {}, expected type: {}, found type: {}", namespace, name, expected, found)
            }
            LinkError::IncorrectMemoryDescriptor{namespace, name,expected,found} => {
                write!(f, "Incorrect memory descriptor, namespace: {}, name: {}, expected memory descriptor: {:?}, found memory descriptor: {:?}", namespace, name, expected, found)
            },
            LinkError::IncorrectTableDescriptor{namespace, name,expected,found} => {
                write!(f, "Incorrect table descriptor, namespace: {}, name: {}, expected table descriptor: {:?}, found table descriptor: {:?}", namespace, name, expected, found)
            },
        }
    }
}

impl std::error::Error for LinkError {}

/// This is the error type returned when calling
/// a WebAssembly function.
///
/// The main way to do this is `Instance.call`.
///
/// Comparing two `RuntimeError`s always evaluates to false.
pub enum RuntimeError {
    Trap { msg: Box<str> },
    Error { data: Box<dyn Any> },
}

impl PartialEq for RuntimeError {
    fn eq(&self, _other: &RuntimeError) -> bool {
        false
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RuntimeError::Trap { ref msg } => {
                write!(f, "WebAssembly trap occurred during runtime: {}", msg)
            }
            RuntimeError::Error { data } => {
                if let Some(s) = data.downcast_ref::<String>() {
                    write!(f, "\"{}\"", s)
                } else if let Some(s) = data.downcast_ref::<&str>() {
                    write!(f, "\"{}\"", s)
                } else {
                    write!(f, "unknown error")
                }
            }
        }
    }
}

impl std::fmt::Debug for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::error::Error for RuntimeError {}

/// This error type is produced by resolving a wasm function
/// given its name.
///
/// Comparing two `ResolveError`s always evaluates to false.
#[derive(Debug, Clone)]
pub enum ResolveError {
    Signature { expected: FuncSig, found: Vec<Type> },
    ExportNotFound { name: String },
    ExportWrongType { name: String },
}

impl PartialEq for ResolveError {
    fn eq(&self, _other: &ResolveError) -> bool {
        false
    }
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ResolveError::ExportNotFound { name } => write!(f, "Export not found: {}", name),
            ResolveError::ExportWrongType { name } => write!(f, "Export wrong type: {}", name),
            ResolveError::Signature { expected, found } => {
                let found = found
                    .as_slice()
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let expected: &FuncSig = expected.borrow();
                write!(
                    f,
                    "Parameters of type [{}] did not match signature {}",
                    found, expected
                )
            }
        }
    }
}

impl std::error::Error for ResolveError {}

/// This error type is produced by calling a wasm function
/// exported from a module.
///
/// If the module traps in some way while running, this will
/// be the `CallError::Runtime(RuntimeError)` variant.
///
/// Comparing two `CallError`s always evaluates to false.
pub enum CallError {
    Resolve(ResolveError),
    Runtime(RuntimeError),
}

impl PartialEq for CallError {
    fn eq(&self, _other: &CallError) -> bool {
        false
    }
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CallError::Resolve(resolve_error) => write!(f, "Call error: {}", resolve_error),
            CallError::Runtime(runtime_error) => write!(f, "Call error: {}", runtime_error),
        }
    }
}

impl std::fmt::Debug for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CallError::Resolve(resolve_err) => write!(f, "ResolveError: {:?}", resolve_err),
            CallError::Runtime(runtime_err) => write!(f, "RuntimeError: {:?}", runtime_err),
        }
    }
}

impl std::error::Error for CallError {}

/// This error type is produced when creating something,
/// like a `Memory` or a `Table`.
#[derive(Debug, Clone)]
pub enum CreationError {
    UnableToCreateMemory,
    UnableToCreateTable,
    InvalidDescriptor(String),
}

impl PartialEq for CreationError {
    fn eq(&self, _other: &CreationError) -> bool {
        false
    }
}

impl std::fmt::Display for CreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CreationError::UnableToCreateMemory => write!(f, "Unable to Create Memory"),
            CreationError::UnableToCreateTable => write!(f, "Unable to Create Table"),
            CreationError::InvalidDescriptor(msg) => write!(
                f,
                "Unable to create because the supplied descriptor is invalid: \"{}\"",
                msg
            ),
        }
    }
}

impl std::error::Error for CreationError {}

/// The amalgamation of all errors that can occur
/// during the compilation, instantiation, or execution
/// of a WebAssembly module.
///
/// Comparing two `Error`s always evaluates to false.
#[derive(Debug)]
pub enum Error {
    CompileError(CompileError),
    LinkError(Vec<LinkError>),
    RuntimeError(RuntimeError),
    ResolveError(ResolveError),
    CallError(CallError),
    CreationError(CreationError),
}

impl PartialEq for Error {
    fn eq(&self, _other: &Error) -> bool {
        false
    }
}

impl From<CompileError> for Error {
    fn from(compile_err: CompileError) -> Self {
        Error::CompileError(compile_err)
    }
}

impl From<RuntimeError> for Error {
    fn from(runtime_err: RuntimeError) -> Self {
        Error::RuntimeError(runtime_err)
    }
}

impl From<ResolveError> for Error {
    fn from(resolve_err: ResolveError) -> Self {
        Error::ResolveError(resolve_err)
    }
}

impl From<CallError> for Error {
    fn from(call_err: CallError) -> Self {
        Error::CallError(call_err)
    }
}

impl From<CreationError> for Error {
    fn from(creation_err: CreationError) -> Self {
        Error::CreationError(creation_err)
    }
}

impl From<Vec<LinkError>> for Error {
    fn from(link_errs: Vec<LinkError>) -> Self {
        Error::LinkError(link_errs)
    }
}

impl From<RuntimeError> for CallError {
    fn from(runtime_err: RuntimeError) -> Self {
        CallError::Runtime(runtime_err)
    }
}

impl From<ResolveError> for CallError {
    fn from(resolve_err: ResolveError) -> Self {
        CallError::Resolve(resolve_err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::CompileError(err) => write!(f, "compile error: {}", err),
            Error::LinkError(errs) => {
                if errs.len() == 1 {
                    write!(f, "link error: {}", errs[0])
                } else {
                    write!(f, "{} link errors:", errs.len())?;
                    for (i, err) in errs.iter().enumerate() {
                        write!(f, " ({} of {}) {}", i + 1, errs.len(), err)?;
                    }
                    Ok(())
                }
            }
            Error::RuntimeError(err) => write!(f, "runtime error: {}", err),
            Error::ResolveError(err) => write!(f, "resolve error: {}", err),
            Error::CallError(err) => write!(f, "call error: {}", err),
            Error::CreationError(err) => write!(f, "creation error: {}", err),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub enum GrowError {
    MemoryGrowError,
    TableGrowError,
    ExceededMaxPages(PageError),
    ExceededMaxPagesForMemory(usize, usize),
    CouldNotProtectMemory(MemoryProtectionError),
    CouldNotCreateMemory(MemoryCreationError),
}

impl std::fmt::Display for GrowError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GrowError::MemoryGrowError => write!(f, "Unable to grow memory"),
            GrowError::TableGrowError => write!(f, "Unable to grow table"),
            GrowError::ExceededMaxPages(e) => write!(f, "Grow Error: {}", e),
            GrowError::ExceededMaxPagesForMemory(left, added) => write!(f, "Failed to add pages because would exceed maximum number of pages for the memory. Left: {}, Added: {}", left, added),
            GrowError::CouldNotCreateMemory(e) => write!(f, "Grow Error: {}", e),
            GrowError::CouldNotProtectMemory(e) => write!(f, "Grow Error: {}", e),
        }
    }
}

impl std::error::Error for GrowError {}

#[derive(Debug)]
pub enum PageError {
    // left, right, added
    ExceededMaxPages(usize, usize, usize),
}

impl std::fmt::Display for PageError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PageError::ExceededMaxPages(left, right, added) => write!(f, "Failed to add pages because would exceed maximum number of pages. Left: {}, Right: {}, Pages added: {}", left, right, added),
        }
    }
}
impl std::error::Error for PageError {}

impl Into<GrowError> for PageError {
    fn into(self) -> GrowError {
        GrowError::ExceededMaxPages(self)
    }
}

#[derive(Debug)]
pub enum MemoryCreationError {
    VirtualMemoryAllocationFailed(usize, String),
    CouldNotCreateMemoryFromFile(std::io::Error),
}

impl std::fmt::Display for MemoryCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MemoryCreationError::VirtualMemoryAllocationFailed(size, msg) => write!(
                f,
                "Allocation virtual memory with size {} failed. \nErrno message: {}",
                size, msg
            ),
            MemoryCreationError::CouldNotCreateMemoryFromFile(e) => write!(f, "IO Error: {}", e),
        }
    }
}
impl std::error::Error for MemoryCreationError {}

impl Into<GrowError> for MemoryCreationError {
    fn into(self) -> GrowError {
        GrowError::CouldNotCreateMemory(self)
    }
}

impl From<std::io::Error> for MemoryCreationError {
    fn from(io_error: std::io::Error) -> Self {
        MemoryCreationError::CouldNotCreateMemoryFromFile(io_error)
    }
}

#[derive(Debug)]
pub enum MemoryProtectionError {
    ProtectionFailed(usize, usize, String),
}

impl std::fmt::Display for MemoryProtectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MemoryProtectionError::ProtectionFailed(start, size, msg) => write!(
                f,
                "Allocation virtual memory starting at {} with size {} failed. \nErrno message: {}",
                start, size, msg
            ),
        }
    }
}
impl std::error::Error for MemoryProtectionError {}

impl Into<GrowError> for MemoryProtectionError {
    fn into(self) -> GrowError {
        GrowError::CouldNotProtectMemory(self)
    }
}

#[derive(Debug)]
pub enum ParseError {
    BinaryReadError,
}

impl From<wasmparser::BinaryReaderError> for ParseError {
    fn from(_: wasmparser::BinaryReaderError) -> Self {
        ParseError::BinaryReadError
    }
}
