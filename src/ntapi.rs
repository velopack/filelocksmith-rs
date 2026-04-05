//! Low-level NT API bindings, undocumented struct definitions, and RAII handle wrapper.

use std::ffi::c_void;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

// ── Constants ──────────────────────────────────────────────────────────────

pub const SYSTEM_EXTENDED_HANDLE_INFORMATION: u32 = 64;
pub const OBJECT_NAME_INFORMATION: u32 = 1;
pub const OBJECT_TYPE_INFORMATION_CLASS: u32 = 2;
pub const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC000_0004_u32 as i32;
pub const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;
pub const MAX_BUFFER_SIZE: usize = 1024 * 1024 * 1024;

// ── Undocumented NT structs ────────────────────────────────────────────────

/// UNICODE_STRING as defined by ntdll. We define our own to avoid extra dependencies.
#[repr(C)]
pub struct UnicodeString {
    /// Length in **bytes** (not characters).
    pub length: u16,
    pub maximum_length: u16,
    pub buffer: *mut u16,
}

#[repr(C)]
pub struct SystemHandleTableEntryInfoEx {
    pub object: *mut c_void,
    pub unique_process_id: usize,
    pub handle_value: usize,
    pub granted_access: u32,
    pub creator_back_trace_index: u16,
    pub object_type_index: u16,
    pub handle_attributes: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct SystemHandleInformationEx {
    pub number_of_handles: usize,
    pub reserved: usize,
    /// Flexible array member — access via pointer arithmetic.
    pub handles: [SystemHandleTableEntryInfoEx; 0],
}

// ── RAII Handle ────────────────────────────────────────────────────────────

/// Owns a Windows `HANDLE` and calls `CloseHandle` on drop.
pub struct SafeHandle(HANDLE);

impl SafeHandle {
    /// Wrap a handle where 0 (NULL) **or** `INVALID_HANDLE_VALUE` means failure.
    pub fn from_raw(handle: HANDLE) -> Option<Self> {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(Self(handle))
        }
    }

    /// Wrap a handle where only `INVALID_HANDLE_VALUE` means failure (e.g. `CreateFileW`).
    pub fn from_raw_file(handle: HANDLE) -> Option<Self> {
        if handle == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(Self(handle))
        }
    }

    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for SafeHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

// ── NT function pointer types ──────────────────────────────────────────────

type NtQuerySystemInformationFn = unsafe extern "system" fn(
    system_information_class: u32,
    system_information: *mut c_void,
    system_information_length: u32,
    return_length: *mut u32,
) -> i32;

type NtQueryObjectFn = unsafe extern "system" fn(
    handle: HANDLE,
    object_information_class: u32,
    object_information: *mut c_void,
    object_information_length: u32,
    return_length: *mut u32,
) -> i32;

/// Resolved function pointers into ntdll.dll.
pub struct NtdllFunctions {
    query_system_information: NtQuerySystemInformationFn,
    query_object: NtQueryObjectFn,
}

impl NtdllFunctions {
    /// Resolve the NT functions from ntdll.dll. Returns `None` if any lookup fails.
    pub fn load() -> Option<Self> {
        // "ntdll.dll\0" as a null-terminated UTF-16 string
        const NTDLL: [u16; 10] = [110, 116, 100, 108, 108, 46, 100, 108, 108, 0];

        unsafe {
            let module = GetModuleHandleW(NTDLL.as_ptr());
            if module.is_null() {
                return None;
            }

            let qsi = GetProcAddress(module, b"NtQuerySystemInformation\0".as_ptr())?;
            let qo = GetProcAddress(module, b"NtQueryObject\0".as_ptr())?;

            Some(Self {
                query_system_information: std::mem::transmute(qsi),
                query_object: std::mem::transmute(qo),
            })
        }
    }

    pub unsafe fn query_system_information(
        &self,
        class: u32,
        buf: *mut c_void,
        len: u32,
        ret_len: *mut u32,
    ) -> i32 {
        (self.query_system_information)(class, buf, len, ret_len)
    }

    pub unsafe fn query_object(
        &self,
        handle: HANDLE,
        class: u32,
        buf: *mut c_void,
        len: u32,
        ret_len: *mut u32,
    ) -> i32 {
        (self.query_object)(handle, class, buf, len, ret_len)
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Buffer-doubling loop for `NtQuerySystemInformation`.
pub fn nt_query_system_information_loop(ntdll: &NtdllFunctions, class: u32) -> Option<Vec<u8>> {
    let mut size = DEFAULT_BUFFER_SIZE;
    loop {
        let mut buffer = vec![0u8; size];
        let mut return_length = 0u32;
        let status = unsafe {
            ntdll.query_system_information(
                class,
                buffer.as_mut_ptr().cast(),
                size as u32,
                &mut return_length,
            )
        };

        if status == STATUS_INFO_LENGTH_MISMATCH {
            size *= 2;
            if size > MAX_BUFFER_SIZE {
                return None;
            }
            continue;
        }

        if status < 0 {
            return None;
        }

        return Some(buffer);
    }
}

/// Read a `UNICODE_STRING` from a raw pointer into a `Vec<u16>`.
pub unsafe fn read_unicode_string(ptr: *const UnicodeString) -> Vec<u16> {
    let us = &*ptr;
    if us.buffer.is_null() || us.length == 0 {
        return Vec::new();
    }
    let char_count = us.length as usize / 2;
    std::slice::from_raw_parts(us.buffer, char_count).to_vec()
}
