//! Direct Win32 / NT bindings used by this crate.
//!
//! Re-declared here so we don't need a dependency on `windows-sys`. Only the
//! small subset of types, constants, and functions we actually call is
//! present; everything is `#[repr(C)]`-compatible with the corresponding
//! Win32 definitions.

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

use std::ffi::c_void;

// ── Type aliases ───────────────────────────────────────────────────────────

pub type HANDLE = *mut c_void;
pub type HMODULE = *mut c_void;
pub type BOOL = i32;
pub type FARPROC = Option<unsafe extern "system" fn() -> isize>;

// ── Constants ──────────────────────────────────────────────────────────────

pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

pub const ERROR_NOT_ALL_ASSIGNED: u32 = 1300;
pub const WAIT_OBJECT_0: u32 = 0;

pub const TOKEN_QUERY: u32 = 0x0008;
pub const TOKEN_ADJUST_PRIVILEGES: u32 = 0x0020;
pub const SE_PRIVILEGE_ENABLED: u32 = 0x0000_0002;

/// `TOKEN_INFORMATION_CLASS::TokenElevation` value.
pub const TokenElevation: i32 = 20;

pub const PROCESS_TERMINATE: u32 = 0x0001;
pub const PROCESS_VM_READ: u32 = 0x0010;
pub const PROCESS_DUP_HANDLE: u32 = 0x0040;
pub const PROCESS_QUERY_INFORMATION: u32 = 0x0400;

pub const DUPLICATE_SAME_ACCESS: u32 = 0x0000_0002;

pub const FILE_SHARE_READ: u32 = 0x0000_0001;
pub const FILE_SHARE_WRITE: u32 = 0x0000_0002;
pub const FILE_SHARE_DELETE: u32 = 0x0000_0004;
pub const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
pub const OPEN_EXISTING: u32 = 3;
pub const FILE_TYPE_DISK: u32 = 0x0001;

// ── Structures ─────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone)]
pub struct LUID {
    pub LowPart: u32,
    pub HighPart: i32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct LUID_AND_ATTRIBUTES {
    pub Luid: LUID,
    pub Attributes: u32,
}

#[repr(C)]
pub struct TOKEN_ELEVATION {
    pub TokenIsElevated: u32,
}

#[repr(C)]
pub struct TOKEN_PRIVILEGES {
    pub PrivilegeCount: u32,
    pub Privileges: [LUID_AND_ATTRIBUTES; 1],
}

// ── Function imports ───────────────────────────────────────────────────────

#[link(name = "kernel32")]
extern "system" {
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;
    pub fn GetLastError() -> u32;
    pub fn DuplicateHandle(
        hSourceProcessHandle: HANDLE,
        hSourceHandle: HANDLE,
        hTargetProcessHandle: HANDLE,
        lpTargetHandle: *mut HANDLE,
        dwDesiredAccess: u32,
        bInheritHandle: BOOL,
        dwOptions: u32,
    ) -> BOOL;
    pub fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *const c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: HANDLE,
    ) -> HANDLE;
    pub fn GetFileType(hFile: HANDLE) -> u32;
    pub fn CreateThread(
        lpThreadAttributes: *const c_void,
        dwStackSize: usize,
        lpStartAddress: Option<unsafe extern "system" fn(*mut c_void) -> u32>,
        lpParameter: *const c_void,
        dwCreationFlags: u32,
        lpThreadId: *mut u32,
    ) -> HANDLE;
    pub fn GetCurrentProcess() -> HANDLE;
    pub fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: BOOL, dwProcessId: u32) -> HANDLE;
    pub fn TerminateThread(hThread: HANDLE, dwExitCode: u32) -> BOOL;
    pub fn TerminateProcess(hProcess: HANDLE, uExitCode: u32) -> BOOL;
    pub fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: u32) -> u32;
    pub fn GetModuleHandleW(lpModuleName: *const u16) -> HMODULE;
    pub fn GetProcAddress(hModule: HMODULE, lpProcName: *const u8) -> FARPROC;

    // The K32-prefixed exports live in kernel32.dll on every supported
    // Windows version, so we don't need to link psapi.lib separately.
    #[link_name = "K32EnumProcesses"]
    pub fn EnumProcesses(lpidProcess: *mut u32, cb: u32, lpcbNeeded: *mut u32) -> BOOL;
    #[link_name = "K32EnumProcessModules"]
    pub fn EnumProcessModules(
        hProcess: HANDLE,
        lphModule: *mut HMODULE,
        cb: u32,
        lpcbNeeded: *mut u32,
    ) -> BOOL;
    #[link_name = "K32GetModuleFileNameExW"]
    pub fn GetModuleFileNameExW(
        hProcess: HANDLE,
        hModule: HMODULE,
        lpFilename: *mut u16,
        nSize: u32,
    ) -> u32;
}

#[link(name = "advapi32")]
extern "system" {
    pub fn OpenProcessToken(
        ProcessHandle: HANDLE,
        DesiredAccess: u32,
        TokenHandle: *mut HANDLE,
    ) -> BOOL;
    pub fn GetTokenInformation(
        TokenHandle: HANDLE,
        TokenInformationClass: i32,
        TokenInformation: *mut c_void,
        TokenInformationLength: u32,
        ReturnLength: *mut u32,
    ) -> BOOL;
    pub fn AdjustTokenPrivileges(
        TokenHandle: HANDLE,
        DisableAllPrivileges: BOOL,
        NewState: *const TOKEN_PRIVILEGES,
        BufferLength: u32,
        PreviousState: *mut TOKEN_PRIVILEGES,
        ReturnLength: *mut u32,
    ) -> BOOL;
    pub fn LookupPrivilegeValueW(
        lpSystemName: *const u16,
        lpName: *const u16,
        lpLuid: *mut LUID,
    ) -> BOOL;
}
