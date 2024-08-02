use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

extern "C" {
    fn PidToFullPath(pid: usize) -> *mut c_char;
    fn FreeString(ptr: *mut c_char);
    fn IsProcessElevated() -> bool;
    fn SetDebugPrivilege() -> bool;
}

pub struct ProcessInfo {
    pub name: String,
    pub pid: usize,
}

// pub fn find_processes_locking_path<P: AsRef<Path>>(path: P) -> Vec<ProcessInfo> {}

// pub fn quit_processes(locked_processes: Vec<ProcessInfo>) -> bool {}

pub fn is_process_elevated() -> bool {
    unsafe { IsProcessElevated() }
}

pub fn set_debug_privilege() -> bool {
    unsafe { SetDebugPrivilege() }
}

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
