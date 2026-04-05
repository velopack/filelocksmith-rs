//! # filelocksmith-rs
//! [![Version](https://img.shields.io/crates/v/filelocksmith?style=flat-square)](https://crates.io/crates/filelocksmith)
//! [![License](https://img.shields.io/crates/l/filelocksmith?style=flat-square)](https://github.com/velopack/filelocksmith-rs/blob/master/LICENSE)
//! [![Documentation](https://img.shields.io/docsrs/filelocksmith?style=flat-square)](https://docs.rs/filelocksmith/latest/filelocksmith/)
//!
//! Reliably find and quit processes that are locking a file or folder on Windows.
//! This is a difficult problem to solve on Windows, as the OS does not provide a built-in or
//! straight-forward way to do this.
//!
//! Additionally, unlike *nix, files and folders can not be deleted or moved while they are locked by a process.
//!
//! ## Installing
//! ```toml
//! [dependencies]
//! filelocksmith = "0.1"
//! ```
//!
//! ## Usage
//! ```rust
//! use filelocksmith::{find_processes_locking_path, quit_processes, pid_to_process_path};
//!
//! let path = "C:\\path\\to\\file.txt";
//! let pids = find_processes_locking_path(path);
//!
//! // print paths of processes locking the file
//! for pid in &pids {
//!    println!("[{}] {:?}", pid, pid_to_process_path(*pid));
//! }
//!
//! // quit the processes locking the file
//! if quit_processes(pids) {
//!  println!("Processes quit successfully");
//! }
//! ```

#![warn(missing_docs)]

#[cfg(windows)]
mod handles;
#[cfg(windows)]
mod ntapi;

use std::path::Path;

/// Find processes locking a file or folder. Returns a list of process IDs.
///
/// If the current Rust process is not running as administrator, but the locking process is,
/// the locking process will not be detected.
///
/// You can use `is_process_elevated` to check if the current process is running as administrator.
/// Also, you can use `set_debug_privilege` to set the SeDebugPrivilege, which will allow the
/// current process to detect all processes on the system. Note that `set_debug_privilege` also
/// requires administrator privileges to work.
pub fn find_processes_locking_path<P: AsRef<Path>>(path: P) -> Vec<usize> {
    #[cfg(windows)]
    {
        handles::find_processes_for_paths(&[path.as_ref()])
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Vec::new()
    }
}

/// Returns true if the current process is running as administrator.
pub fn is_process_elevated() -> bool {
    #[cfg(windows)]
    {
        use std::ptr;
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::Security::{
            GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        #[link(name = "advapi32")]
        extern "system" {
            fn OpenProcessToken(
                process_handle: HANDLE,
                desired_access: u32,
                token_handle: *mut HANDLE,
            ) -> i32;
        }

        unsafe {
            let mut token: HANDLE = ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return false;
            }
            let mut elevation = TOKEN_ELEVATION {
                TokenIsElevated: 0,
            };
            let mut size = 0u32;
            let ok = GetTokenInformation(
                token,
                TokenElevation,
                (&mut elevation as *mut TOKEN_ELEVATION).cast(),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            );
            CloseHandle(token);
            ok != 0 && elevation.TokenIsElevated != 0
        }
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Sets the SeDebugPrivilege for the current process. Returns true if successful.
pub fn set_debug_privilege() -> bool {
    #[cfg(windows)]
    {
        use std::ptr;
        use windows_sys::Win32::Foundation::{
            CloseHandle, GetLastError, HANDLE, ERROR_NOT_ALL_ASSIGNED,
        };
        use windows_sys::Win32::Security::{
            AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES,
            SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        #[link(name = "advapi32")]
        extern "system" {
            fn OpenProcessToken(
                process_handle: HANDLE,
                desired_access: u32,
                token_handle: *mut HANDLE,
            ) -> i32;
        }

        // "SeDebugPrivilege\0"
        const SE_DEBUG_NAME: [u16; 17] = [
            83, 101, 68, 101, 98, 117, 103, 80, 114, 105, 118, 105, 108, 101, 103, 101, 0,
        ];

        unsafe {
            let mut token: HANDLE = ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES, &mut token) == 0 {
                return false;
            }

            let mut luid = std::mem::zeroed();
            if LookupPrivilegeValueW(ptr::null(), SE_DEBUG_NAME.as_ptr(), &mut luid) == 0 {
                CloseHandle(token);
                return false;
            }

            let tp = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES {
                    Luid: luid,
                    Attributes: SE_PRIVILEGE_ENABLED,
                }],
            };

            let ok = AdjustTokenPrivileges(
                token,
                0,
                &tp,
                std::mem::size_of::<TOKEN_PRIVILEGES>() as u32,
                ptr::null_mut(),
                ptr::null_mut(),
            );

            let failed = ok == 0 || GetLastError() == ERROR_NOT_ALL_ASSIGNED;
            CloseHandle(token);
            !failed
        }
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Returns the full path of a process given its process ID. If the process is not found,
/// or we do not have permission to access the process, this function will return None.
pub fn pid_to_process_path(pid: usize) -> Option<String> {
    #[cfg(windows)]
    {
        use std::ptr;
        use windows_sys::Win32::System::ProcessStatus::GetModuleFileNameExW;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
        };

        let process =
            unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid as u32) };
        let process = ntapi::SafeHandle::from_raw(process)?;

        let mut buf = vec![0u16; 65536];
        let len = unsafe {
            GetModuleFileNameExW(process.raw(), ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32)
        };
        if len == 0 {
            return None;
        }
        buf.truncate(len as usize);
        Some(String::from_utf16_lossy(&buf))
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        None
    }
}

/// Quits processes given a list of process IDs. Returns true if all processes were quit
/// successfully, or there were no processes to quit. If one or more processes could not be quit,
/// this function will return false.
pub fn quit_processes(pids: Vec<usize>) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, TerminateProcess, PROCESS_TERMINATE,
        };

        let mut ok = true;
        for pid in pids {
            unsafe {
                let process = OpenProcess(PROCESS_TERMINATE, 0, pid as u32);
                if process.is_null() {
                    ok = false;
                    continue;
                }
                TerminateProcess(process, 0);
                CloseHandle(process);
            }
        }
        ok
    }
    #[cfg(not(windows))]
    {
        let _ = pids;
        true
    }
}
