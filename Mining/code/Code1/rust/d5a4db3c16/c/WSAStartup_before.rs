    pub fn WSAStartup(wVersionRequested: WORD,
                      lpWSAData: LPWSADATA) -> c_int;
    pub fn WSACleanup() -> c_int;
    pub fn WSAGetLastError() -> c_int;
    pub fn WSADuplicateSocketW(s: SOCKET,
                               dwProcessId: DWORD,
                               lpProtocolInfo: LPWSAPROTOCOL_INFO)
                               -> c_int;
    pub fn GetCurrentProcessId() -> DWORD;
    pub fn WSASocketW(af: c_int,
                      kind: c_int,
                      protocol: c_int,
                      lpProtocolInfo: LPWSAPROTOCOL_INFO,
                      g: GROUP,
                      dwFlags: DWORD) -> SOCKET;
    pub fn ioctlsocket(s: SOCKET, cmd: c_long, argp: *mut c_ulong) -> c_int;
    pub fn InitializeCriticalSection(CriticalSection: *mut CRITICAL_SECTION);
    pub fn EnterCriticalSection(CriticalSection: *mut CRITICAL_SECTION);
    pub fn TryEnterCriticalSection(CriticalSection: *mut CRITICAL_SECTION) -> BOOLEAN;
    pub fn LeaveCriticalSection(CriticalSection: *mut CRITICAL_SECTION);
    pub fn DeleteCriticalSection(CriticalSection: *mut CRITICAL_SECTION);

    pub fn ReadConsoleW(hConsoleInput: HANDLE,
                        lpBuffer: LPVOID,
                        nNumberOfCharsToRead: DWORD,
                        lpNumberOfCharsRead: LPDWORD,
                        pInputControl: PCONSOLE_READCONSOLE_CONTROL) -> BOOL;

    pub fn WriteConsoleW(hConsoleOutput: HANDLE,
                         lpBuffer: LPCVOID,
                         nNumberOfCharsToWrite: DWORD,
                         lpNumberOfCharsWritten: LPDWORD,
                         lpReserved: LPVOID) -> BOOL;

    pub fn GetConsoleMode(hConsoleHandle: HANDLE,
                          lpMode: LPDWORD) -> BOOL;
    pub fn RemoveDirectoryW(lpPathName: LPCWSTR) -> BOOL;
    pub fn SetFileAttributesW(lpFileName: LPCWSTR,
                              dwFileAttributes: DWORD) -> BOOL;
    pub fn GetFileInformationByHandle(hFile: HANDLE,
                            lpFileInformation: LPBY_HANDLE_FILE_INFORMATION)
                            -> BOOL;

    pub fn SetLastError(dwErrCode: DWORD);
    pub fn GetCommandLineW() -> *mut LPCWSTR;
    pub fn LocalFree(ptr: *mut c_void);
    pub fn CommandLineToArgvW(lpCmdLine: *mut LPCWSTR,
                              pNumArgs: *mut c_int) -> *mut *mut u16;
    pub fn GetTempPathW(nBufferLength: DWORD,
                        lpBuffer: LPCWSTR) -> DWORD;
    pub fn OpenProcessToken(ProcessHandle: HANDLE,
                            DesiredAccess: DWORD,
                            TokenHandle: *mut HANDLE) -> BOOL;
    pub fn GetCurrentProcess() -> HANDLE;
    pub fn GetCurrentThread() -> HANDLE;
    pub fn GetStdHandle(which: DWORD) -> HANDLE;
    pub fn ExitProcess(uExitCode: c_uint) -> !;
    pub fn DeviceIoControl(hDevice: HANDLE,
                           dwIoControlCode: DWORD,
                           lpInBuffer: LPVOID,
                           nInBufferSize: DWORD,
                           lpOutBuffer: LPVOID,
                           nOutBufferSize: DWORD,
                           lpBytesReturned: LPDWORD,
                           lpOverlapped: LPOVERLAPPED) -> BOOL;
    pub fn CreateThread(lpThreadAttributes: LPSECURITY_ATTRIBUTES,
                        dwStackSize: SIZE_T,
                        lpStartAddress: extern "system" fn(*mut c_void)
                                                           -> DWORD,
                        lpParameter: LPVOID,
                        dwCreationFlags: DWORD,
                        lpThreadId: LPDWORD) -> HANDLE;
    pub fn WaitForSingleObject(hHandle: HANDLE,
                               dwMilliseconds: DWORD) -> DWORD;
    pub fn SwitchToThread() -> BOOL;
    pub fn Sleep(dwMilliseconds: DWORD);
    pub fn GetProcessId(handle: HANDLE) -> DWORD;
    pub fn GetUserProfileDirectoryW(hToken: HANDLE,
                                    lpProfileDir: LPCWSTR,
                                    lpcchSize: *mut DWORD) -> BOOL;
    pub fn SetHandleInformation(hObject: HANDLE,
                                dwMask: DWORD,
                                dwFlags: DWORD) -> BOOL;
    pub fn CopyFileExW(lpExistingFileName: LPCWSTR,
                       lpNewFileName: LPCWSTR,
                       lpProgressRoutine: LPPROGRESS_ROUTINE,
                       lpData: LPVOID,
                       pbCancel: LPBOOL,
                       dwCopyFlags: DWORD) -> BOOL;
    pub fn AddVectoredExceptionHandler(FirstHandler: ULONG,
                                       VectoredHandler: PVECTORED_EXCEPTION_HANDLER)
                                       -> LPVOID;
    pub fn FormatMessageW(flags: DWORD,
                          lpSrc: LPVOID,
                          msgId: DWORD,
                          langId: DWORD,
                          buf: LPWSTR,
                          nsize: DWORD,
                          args: *const c_void)
                          -> DWORD;
    pub fn TlsAlloc() -> DWORD;
    pub fn TlsFree(dwTlsIndex: DWORD) -> BOOL;
    pub fn TlsGetValue(dwTlsIndex: DWORD) -> LPVOID;
    pub fn TlsSetValue(dwTlsIndex: DWORD, lpTlsvalue: LPVOID) -> BOOL;
    pub fn GetLastError() -> DWORD;
    pub fn QueryPerformanceFrequency(lpFrequency: *mut LARGE_INTEGER) -> BOOL;
    pub fn QueryPerformanceCounter(lpPerformanceCount: *mut LARGE_INTEGER)
                                   -> BOOL;
    pub fn GetExitCodeProcess(hProcess: HANDLE, lpExitCode: LPDWORD) -> BOOL;
    pub fn TerminateProcess(hProcess: HANDLE, uExitCode: UINT) -> BOOL;
    pub fn CreateProcessW(lpApplicationName: LPCWSTR,
                          lpCommandLine: LPWSTR,
                          lpProcessAttributes: LPSECURITY_ATTRIBUTES,
                          lpThreadAttributes: LPSECURITY_ATTRIBUTES,
                          bInheritHandles: BOOL,
                          dwCreationFlags: DWORD,
                          lpEnvironment: LPVOID,
                          lpCurrentDirectory: LPCWSTR,
                          lpStartupInfo: LPSTARTUPINFO,
                          lpProcessInformation: LPPROCESS_INFORMATION)
                          -> BOOL;
    pub fn GetEnvironmentVariableW(n: LPCWSTR, v: LPWSTR, nsize: DWORD) -> DWORD;
    pub fn SetEnvironmentVariableW(n: LPCWSTR, v: LPCWSTR) -> BOOL;
    pub fn GetEnvironmentStringsW() -> LPWCH;
    pub fn FreeEnvironmentStringsW(env_ptr: LPWCH) -> BOOL;
    pub fn GetModuleFileNameW(hModule: HMODULE,
                              lpFilename: LPWSTR,
                              nSize: DWORD)
                              -> DWORD;
    pub fn CreateDirectoryW(lpPathName: LPCWSTR,
                            lpSecurityAttributes: LPSECURITY_ATTRIBUTES)
                            -> BOOL;
    pub fn DeleteFileW(lpPathName: LPCWSTR) -> BOOL;
    pub fn GetCurrentDirectoryW(nBufferLength: DWORD, lpBuffer: LPWSTR) -> DWORD;
    pub fn SetCurrentDirectoryW(lpPathName: LPCWSTR) -> BOOL;
    pub fn WideCharToMultiByte(CodePage: UINT,
                               dwFlags: DWORD,
                               lpWideCharStr: LPCWSTR,
                               cchWideChar: c_int,
                               lpMultiByteStr: LPSTR,
                               cbMultiByte: c_int,
                               lpDefaultChar: LPCSTR,
                               lpUsedDefaultChar: LPBOOL) -> c_int;

    pub fn closesocket(socket: SOCKET) -> c_int;
    pub fn recv(socket: SOCKET, buf: *mut c_void, len: c_int,
                flags: c_int) -> c_int;
    pub fn send(socket: SOCKET, buf: *const c_void, len: c_int,
                flags: c_int) -> c_int;
    pub fn recvfrom(socket: SOCKET,
                    buf: *mut c_void,
                    len: c_int,
                    flags: c_int,
                    addr: *mut SOCKADDR,
                    addrlen: *mut c_int)
                    -> c_int;
    pub fn sendto(socket: SOCKET,
                  buf: *const c_void,
                  len: c_int,
                  flags: c_int,
                  addr: *const SOCKADDR,
                  addrlen: c_int)
                  -> c_int;
    pub fn shutdown(socket: SOCKET, how: c_int) -> c_int;
    pub fn accept(socket: SOCKET,
                  address: *mut SOCKADDR,
                  address_len: *mut c_int)
                  -> SOCKET;
    pub fn DuplicateHandle(hSourceProcessHandle: HANDLE,
                           hSourceHandle: HANDLE,
                           hTargetProcessHandle: HANDLE,
                           lpTargetHandle: LPHANDLE,
                           dwDesiredAccess: DWORD,
                           bInheritHandle: BOOL,
                           dwOptions: DWORD)
                           -> BOOL;
    pub fn ReadFile(hFile: HANDLE,
                    lpBuffer: LPVOID,
                    nNumberOfBytesToRead: DWORD,
                    lpNumberOfBytesRead: LPDWORD,
                    lpOverlapped: LPOVERLAPPED)
                    -> BOOL;
    pub fn WriteFile(hFile: HANDLE,
                     lpBuffer: LPVOID,
                     nNumberOfBytesToWrite: DWORD,
                     lpNumberOfBytesWritten: LPDWORD,
                     lpOverlapped: LPOVERLAPPED)
                     -> BOOL;
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;
    pub fn CreateHardLinkW(lpSymlinkFileName: LPCWSTR,
                           lpTargetFileName: LPCWSTR,
                           lpSecurityAttributes: LPSECURITY_ATTRIBUTES)
                           -> BOOL;
    pub fn MoveFileExW(lpExistingFileName: LPCWSTR,
                       lpNewFileName: LPCWSTR,
                       dwFlags: DWORD)
                       -> BOOL;
    pub fn SetFilePointerEx(hFile: HANDLE,
                            liDistanceToMove: LARGE_INTEGER,
                            lpNewFilePointer: PLARGE_INTEGER,
                            dwMoveMethod: DWORD)
                            -> BOOL;
    pub fn FlushFileBuffers(hFile: HANDLE) -> BOOL;
    pub fn CreateFileW(lpFileName: LPCWSTR,
                       dwDesiredAccess: DWORD,
                       dwShareMode: DWORD,
                       lpSecurityAttributes: LPSECURITY_ATTRIBUTES,
                       dwCreationDisposition: DWORD,
                       dwFlagsAndAttributes: DWORD,
                       hTemplateFile: HANDLE)
                       -> HANDLE;

    pub fn FindFirstFileW(fileName: LPCWSTR,
                          findFileData: LPWIN32_FIND_DATAW)
                          -> HANDLE;
    pub fn FindNextFileW(findFile: HANDLE, findFileData: LPWIN32_FIND_DATAW)
                         -> BOOL;
    pub fn FindClose(findFile: HANDLE) -> BOOL;
    pub fn RtlCaptureContext(ctx: *mut CONTEXT);
    pub fn getsockopt(s: SOCKET,
                      level: c_int,
                      optname: c_int,
                      optval: *mut c_char,
                      optlen: *mut c_int)
                      -> c_int;
    pub fn setsockopt(s: SOCKET,
                      level: c_int,
                      optname: c_int,
                      optval: *const c_void,
                      optlen: c_int)
                      -> c_int;
    pub fn getsockname(socket: SOCKET,
                       address: *mut SOCKADDR,
                       address_len: *mut c_int)
                       -> c_int;
    pub fn getpeername(socket: SOCKET,
                       address: *mut SOCKADDR,
                       address_len: *mut c_int)
                       -> c_int;
    pub fn bind(socket: SOCKET, address: *const SOCKADDR,
                address_len: socklen_t) -> c_int;
    pub fn listen(socket: SOCKET, backlog: c_int) -> c_int;
    pub fn connect(socket: SOCKET, address: *const SOCKADDR, len: c_int)
                   -> c_int;
    pub fn getaddrinfo(node: *const c_char, service: *const c_char,
                       hints: *const ADDRINFOA,
                       res: *mut *mut ADDRINFOA) -> c_int;
    pub fn freeaddrinfo(res: *mut ADDRINFOA);

    pub fn LoadLibraryW(name: LPCWSTR) -> HMODULE;
    pub fn FreeLibrary(handle: HMODULE) -> BOOL;
    pub fn GetProcAddress(handle: HMODULE,
                          name: LPCSTR) -> *mut c_void;
    pub fn GetModuleHandleW(lpModuleName: LPCWSTR) -> HMODULE;
    pub fn CryptAcquireContextA(phProv: *mut HCRYPTPROV,
                                pszContainer: LPCSTR,
                                pszProvider: LPCSTR,
                                dwProvType: DWORD,
                                dwFlags: DWORD) -> BOOL;
    pub fn CryptGenRandom(hProv: HCRYPTPROV,
                          dwLen: DWORD,
                          pbBuffer: *mut BYTE) -> BOOL;
    pub fn CryptReleaseContext(hProv: HCRYPTPROV, dwFlags: DWORD) -> BOOL;

    pub fn GetSystemTimeAsFileTime(lpSystemTimeAsFileTime: LPFILETIME);

    pub fn CreateEventW(lpEventAttributes: LPSECURITY_ATTRIBUTES,
                        bManualReset: BOOL,
                        bInitialState: BOOL,
                        lpName: LPCWSTR) -> HANDLE;
    pub fn WaitForMultipleObjects(nCount: DWORD,
                                  lpHandles: *const HANDLE,
                                  bWaitAll: BOOL,
                                  dwMilliseconds: DWORD) -> DWORD;
    pub fn CreateNamedPipeW(lpName: LPCWSTR,
                            dwOpenMode: DWORD,
                            dwPipeMode: DWORD,
                            nMaxInstances: DWORD,
                            nOutBufferSize: DWORD,
                            nInBufferSize: DWORD,
                            nDefaultTimeOut: DWORD,
                            lpSecurityAttributes: LPSECURITY_ATTRIBUTES)
                            -> HANDLE;
    pub fn CancelIo(handle: HANDLE) -> BOOL;
    pub fn GetOverlappedResult(hFile: HANDLE,
                               lpOverlapped: LPOVERLAPPED,
                               lpNumberOfBytesTransferred: LPDWORD,
                               bWait: BOOL) -> BOOL;
}

// Functions that aren't available on Windows XP, but we still use them and just
// provide some form of a fallback implementation.
compat_fn! {
    kernel32:

    pub fn CreateSymbolicLinkW(_lpSymlinkFileName: LPCWSTR,
                               _lpTargetFileName: LPCWSTR,
                               _dwFlags: DWORD) -> BOOLEAN {
        SetLastError(ERROR_CALL_NOT_IMPLEMENTED as DWORD); 0
    }
    pub fn GetFinalPathNameByHandleW(_hFile: HANDLE,
                                     _lpszFilePath: LPCWSTR,
                                     _cchFilePath: DWORD,
                                     _dwFlags: DWORD) -> DWORD {
        SetLastError(ERROR_CALL_NOT_IMPLEMENTED as DWORD); 0
    }
    pub fn SetThreadStackGuarantee(_size: *mut c_ulong) -> BOOL {
        SetLastError(ERROR_CALL_NOT_IMPLEMENTED as DWORD); 0
    }
    pub fn SetFileInformationByHandle(_hFile: HANDLE,
                    _FileInformationClass: FILE_INFO_BY_HANDLE_CLASS,
                    _lpFileInformation: LPVOID,
                    _dwBufferSize: DWORD) -> BOOL {
        SetLastError(ERROR_CALL_NOT_IMPLEMENTED as DWORD); 0
    }
    pub fn SleepConditionVariableSRW(ConditionVariable: PCONDITION_VARIABLE,
                                     SRWLock: PSRWLOCK,
                                     dwMilliseconds: DWORD,
                                     Flags: ULONG) -> BOOL {
        panic!("condition variables not available")
    }
    pub fn WakeConditionVariable(ConditionVariable: PCONDITION_VARIABLE)
                                 -> () {
        panic!("condition variables not available")
    }
    pub fn WakeAllConditionVariable(ConditionVariable: PCONDITION_VARIABLE)
                                    -> () {
        panic!("condition variables not available")
    }
    pub fn AcquireSRWLockExclusive(SRWLock: PSRWLOCK) -> () {
        panic!("rwlocks not available")
    }
    pub fn AcquireSRWLockShared(SRWLock: PSRWLOCK) -> () {
        panic!("rwlocks not available")
    }
    pub fn ReleaseSRWLockExclusive(SRWLock: PSRWLOCK) -> () {
        panic!("rwlocks not available")
    }
    pub fn ReleaseSRWLockShared(SRWLock: PSRWLOCK) -> () {
        panic!("rwlocks not available")
    }
    pub fn TryAcquireSRWLockExclusive(SRWLock: PSRWLOCK) -> BOOLEAN {
        panic!("rwlocks not available")
    }
    pub fn TryAcquireSRWLockShared(SRWLock: PSRWLOCK) -> BOOLEAN {
        panic!("rwlocks not available")
    }
}
