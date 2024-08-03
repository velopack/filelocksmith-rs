//! # filelocksmith-rs
//! [![Version](https://img.shields.io/crates/v/filelocksmith?style=flat-square)](https://crates.io/crates/filelocksmith)
//! [![License](https://img.shields.io/crates/l/filelocksmith?style=flat-square)](https://github.com/velopack/filelocksmith-rs/blob/master/LICENSE)
//!
//! Reliably find and quit processes that are locking a file or folder on Windows.
//! This is a difficult problem to solve on Windows, as the OS does not provide a built-in or 
//! straight-forward way to do this.
//!
//! This library wraps the FileLocksmith module from the PowerToys project, which is written in C++.
//! The implementation in PowerToys is endorsed by Microsoft and is very robust. 
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

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::path::Path;

extern "C" {
    fn PidToFullPath(pid: usize) -> *mut c_char;
    fn FreeString(ptr: *mut c_char);
    fn FreeArray(ptr: *mut usize);
    fn IsProcessElevated() -> bool;
    fn SetDebugPrivilege() -> bool;
    fn QuitProcesses(pids: *const usize, count: usize) -> bool;
    fn FindProcessesLockingPath(path_utf8: *mut c_char, pids: *mut *mut usize, count: *mut usize);
}

/// Find processes locking a file or folder. Returns a list of process IDs. 
/// If the current Rust process is not running as administrator, but the locking process is,
/// the locking process will not be detected. You can use `is_process_elevated` to check if the
/// current process is running as administrator. Also, you can use `set_debug_privilege` to set
/// the SeDebugPrivilege, which will allow the current process to detect all processes on the
/// system. Note that `set_debug_privilege` also requires administrator privileges to work.
pub fn find_processes_locking_path<P: AsRef<Path>>(path: P) -> Vec<usize> {
    let path = path.as_ref().as_os_str().as_encoded_bytes();
    let c_path = CString::new(path).expect("CString::new failed");
    let mut pids: *mut usize = ptr::null_mut();
    let mut count: usize = 0;

    unsafe {
        FindProcessesLockingPath(c_path.as_ptr() as *mut c_char, &mut pids, &mut count);

        if !pids.is_null() && count > 0 {
            let slice = std::slice::from_raw_parts(pids, count);
            let pid_vec = slice.to_vec();
            FreeArray(pids);
            pid_vec
        } else {
            vec![]
        }
    }
}

/// Returns true if the current process is running as administrator.
pub fn is_process_elevated() -> bool {
    unsafe { IsProcessElevated() }
}

/// Sets the SeDebugPrivilege for the current process. Returns true if successful.
pub fn set_debug_privilege() -> bool {
    unsafe { SetDebugPrivilege() }
}

/// Returns the full path of a process given its process ID. If the process is not found,
/// or we do not have permission to access the process, this function will return None.
pub fn pid_to_process_path(pid: usize) -> Option<String> {
    unsafe {
        let c_str_ptr = PidToFullPath(pid);
        if c_str_ptr.is_null() {
            None
        } else {
            let c_str = CStr::from_ptr(c_str_ptr);
            let r_str = c_str.to_string_lossy().into_owned();
            FreeString(c_str_ptr);
            Some(r_str)
        }
    }
}

/// Quits processes given a list of process IDs. Returns true if all processes were quit 
/// successfully. If one or more processes could not be quit, this function will return false.
pub fn quit_processes(pids: Vec<usize>) -> bool {
    unsafe {
        QuitProcesses(pids.as_ptr(), pids.len())
    }
}
