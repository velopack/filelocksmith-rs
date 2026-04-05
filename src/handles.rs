//! Handle enumeration with hang detection, kernel-name resolution, and process module scanning.

use std::cell::UnsafeCell;
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::{
    CloseHandle, DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE, WAIT_OBJECT_0,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetFileType, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FILE_TYPE_DISK, OPEN_EXISTING,
};
use windows_sys::Win32::System::ProcessStatus::{
    EnumProcessModules, EnumProcesses, GetModuleFileNameExW,
};
use windows_sys::Win32::System::Threading::{
    CreateThread, GetCurrentProcess, OpenProcess, TerminateThread, WaitForSingleObject,
    PROCESS_DUP_HANDLE, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};

use crate::ntapi::*;

// ── Types ──────────────────────────────────────────────────────────────────

struct HandleInfo {
    pid: usize,
    kernel_file_name: Vec<u16>,
}

// ── Kernel-name resolution ────────────────────────────────────���────────────

fn path_to_kernel_name(path: &Path, ntdll: &NtdllFunctions) -> Option<Vec<u16>> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            ptr::null_mut(),
        )
    };
    let handle = SafeHandle::from_raw_file(handle)?;
    unsafe {
        if GetFileType(handle.raw()) != FILE_TYPE_DISK {
            return None;
        }
    }
    file_handle_to_kernel_name(handle.raw(), ntdll, &mut vec![0u8; DEFAULT_BUFFER_SIZE])
}

fn file_handle_to_kernel_name(
    handle: HANDLE,
    ntdll: &NtdllFunctions,
    buffer: &mut Vec<u8>,
) -> Option<Vec<u16>> {
    let mut return_length = 0u32;
    let status = unsafe {
        ntdll.query_object(
            handle,
            OBJECT_NAME_INFORMATION,
            buffer.as_mut_ptr().cast(),
            buffer.len() as u32,
            &mut return_length,
        )
    };
    if status < 0 {
        return None;
    }
    let name = unsafe { read_unicode_string(buffer.as_ptr().cast()) };
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

// ── Handle enumeration with hang detection ──────────────────���──────────────

struct SharedState {
    ntdll: *const NtdllFunctions,
    handle_info_ptr: *const SystemHandleInformationEx,
    handle_count: usize,
    current_index: AtomicUsize,
    /// Duplicated handle stored as usize for atomic access. 0 means none.
    current_dup_handle: AtomicUsize,
    results: UnsafeCell<Vec<HandleInfo>>,
    pid_to_handle: UnsafeCell<HashMap<usize, HANDLE>>,
    object_info_buffer: UnsafeCell<Vec<u8>>,
}

unsafe impl Send for SharedState {}
unsafe impl Sync for SharedState {}

unsafe extern "system" fn worker_thread_proc(param: *mut c_void) -> u32 {
    let state = &*(param as *const SharedState);
    let ntdll = &*state.ntdll;
    let info = &*state.handle_info_ptr;
    let results = &mut *state.results.get();
    let pid_to_handle = &mut *state.pid_to_handle.get();
    let buf = &mut *state.object_info_buffer.get();
    let handles_base = info.handles.as_ptr();
    let count = state.handle_count;

    let mut i = state.current_index.load(Ordering::Relaxed);
    while i < count {
        state.current_dup_handle.store(0, Ordering::Relaxed);

        let entry = &*handles_base.add(i);
        let pid = entry.unique_process_id;

        let process_handle = if let Some(&h) = pid_to_handle.get(&pid) {
            h
        } else {
            let h = OpenProcess(PROCESS_DUP_HANDLE, 0, pid as u32);
            if h.is_null() {
                i += 1;
                state.current_index.store(i, Ordering::Relaxed);
                continue;
            }
            pid_to_handle.insert(pid, h);
            h
        };

        let mut local_handle: HANDLE = ptr::null_mut();
        let ok = DuplicateHandle(
            process_handle,
            entry.handle_value as HANDLE,
            GetCurrentProcess(),
            &mut local_handle,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        );
        if ok == 0 {
            i += 1;
            state.current_index.store(i, Ordering::Relaxed);
            continue;
        }
        state
            .current_dup_handle
            .store(local_handle as usize, Ordering::Relaxed);

        let mut ret_len = 0u32;
        let status = ntdll.query_object(
            local_handle,
            OBJECT_TYPE_INFORMATION_CLASS,
            buf.as_mut_ptr().cast(),
            buf.len() as u32,
            &mut ret_len,
        );
        if status < 0 {
            CloseHandle(local_handle);
            state.current_dup_handle.store(0, Ordering::Relaxed);
            i += 1;
            state.current_index.store(i, Ordering::Relaxed);
            continue;
        }

        let type_name = read_unicode_string(buf.as_ptr().cast());
        const FILE_TYPE: [u16; 4] = [b'F' as u16, b'i' as u16, b'l' as u16, b'e' as u16];

        if type_name == FILE_TYPE {
            if GetFileType(local_handle) == FILE_TYPE_DISK {
                if let Some(kernel_name) = file_handle_to_kernel_name(local_handle, ntdll, buf) {
                    results.push(HandleInfo {
                        pid,
                        kernel_file_name: kernel_name,
                    });
                }
            }
        }

        CloseHandle(local_handle);
        state.current_dup_handle.store(0, Ordering::Relaxed);
        i += 1;
        state.current_index.store(i, Ordering::Relaxed);
    }
    0
}

fn enumerate_handles(ntdll: &NtdllFunctions) -> Vec<HandleInfo> {
    let buffer = match nt_query_system_information_loop(ntdll, SYSTEM_EXTENDED_HANDLE_INFORMATION) {
        Some(b) => b,
        None => return Vec::new(),
    };

    let info = buffer.as_ptr() as *const SystemHandleInformationEx;
    let handle_count = unsafe { (*info).number_of_handles };

    let state = SharedState {
        ntdll,
        handle_info_ptr: info,
        handle_count,
        current_index: AtomicUsize::new(0),
        current_dup_handle: AtomicUsize::new(0),
        results: UnsafeCell::new(Vec::new()),
        pid_to_handle: UnsafeCell::new(HashMap::new()),
        object_info_buffer: UnsafeCell::new(vec![0u8; DEFAULT_BUFFER_SIZE]),
    };
    let state_ptr = &state as *const SharedState as *mut c_void;

    let mut i: usize = 0;
    while i < handle_count {
        state.current_index.store(i, Ordering::Relaxed);

        let thread = unsafe {
            CreateThread(
                ptr::null(),
                0,
                Some(worker_thread_proc),
                state_ptr,
                0,
                ptr::null_mut(),
            )
        };
        if thread.is_null() {
            break;
        }

        let mut previous_i = i;
        loop {
            let wait = unsafe { WaitForSingleObject(thread, 200) };
            if wait == WAIT_OBJECT_0 {
                unsafe {
                    CloseHandle(thread);
                }
                i = state.current_index.load(Ordering::Relaxed);
                break;
            }

            let current = state.current_index.load(Ordering::Relaxed);
            if current <= previous_i {
                unsafe {
                    TerminateThread(thread, 1);
                    CloseHandle(thread);
                    let dangling = state.current_dup_handle.load(Ordering::Relaxed) as HANDLE;
                    if !dangling.is_null() {
                        CloseHandle(dangling);
                    }
                }
                i = current + 1;
                break;
            }
            previous_i = current;
        }
    }

    let pid_to_handle = unsafe { &*state.pid_to_handle.get() };
    for &h in pid_to_handle.values() {
        unsafe {
            CloseHandle(h);
        }
    }

    std::mem::take(unsafe { &mut *state.results.get() })
}

// ── Module scanning ────────────────────────────────────────────────────────

fn get_process_modules(pid: u32) -> Vec<Vec<u16>> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
    if process.is_null() {
        return Vec::new();
    }
    let _guard = SafeHandle::from_raw(process);

    let mut modules: Vec<HANDLE> = vec![ptr::null_mut(); 1024];
    let mut cb_needed = 0u32;
    let ok = unsafe {
        EnumProcessModules(
            process,
            modules.as_mut_ptr(),
            (modules.len() * std::mem::size_of::<HANDLE>()) as u32,
            &mut cb_needed,
        )
    };
    if ok == 0 {
        return Vec::new();
    }
    let count = cb_needed as usize / std::mem::size_of::<HANDLE>();
    modules.truncate(count);

    let mut paths = Vec::with_capacity(count);
    let mut buf = vec![0u16; 1024];
    for &hmod in &modules {
        let len =
            unsafe { GetModuleFileNameExW(process, hmod, buf.as_mut_ptr(), buf.len() as u32) };
        if len > 0 {
            paths.push(buf[..len as usize].to_vec());
        }
    }
    paths
}

fn wide_path_to_kernel_name(wide: &[u16], ntdll: &NtdllFunctions) -> Option<Vec<u16>> {
    let mut with_null: Vec<u16> = wide.to_vec();
    with_null.push(0);
    let handle = unsafe {
        CreateFileW(
            with_null.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            ptr::null_mut(),
        )
    };
    let handle = SafeHandle::from_raw_file(handle)?;
    unsafe {
        if GetFileType(handle.raw()) != FILE_TYPE_DISK {
            return None;
        }
    }
    file_handle_to_kernel_name(handle.raw(), ntdll, &mut vec![0u8; DEFAULT_BUFFER_SIZE])
}

// ── Path matching ──────────────────────────────────────────────────────────

fn kernel_paths_match(
    kernel_name: &[u16],
    file_kernel_names: &HashSet<Vec<u16>>,
    dir_kernel_names: &[Vec<u16>],
) -> bool {
    if file_kernel_names.contains(kernel_name) {
        return true;
    }
    for dir in dir_kernel_names {
        if kernel_name == dir.as_slice() {
            return true;
        }
        let needs_sep = !dir.is_empty() && *dir.last().unwrap() != b'\\' as u16;
        if needs_sep {
            if kernel_name.len() > dir.len() + 1
                && kernel_name.starts_with(dir)
                && kernel_name[dir.len()] == b'\\' as u16
            {
                return true;
            }
        } else if kernel_name.len() > dir.len() && kernel_name.starts_with(dir) {
            return true;
        }
    }
    false
}

// ── Public entry point ─────────────────────────────────────────────────────

pub fn find_processes_for_paths(paths: &[&Path]) -> Vec<usize> {
    let ntdll = match NtdllFunctions::load() {
        Some(n) => n,
        None => return Vec::new(),
    };

    let mut file_kernel_names: HashSet<Vec<u16>> = HashSet::new();
    let mut dir_kernel_names: Vec<Vec<u16>> = Vec::new();

    for &path in paths {
        if let Some(kn) = path_to_kernel_name(path, &ntdll) {
            if path.is_dir() {
                dir_kernel_names.push(kn);
            } else {
                file_kernel_names.insert(kn);
            }
        }
    }

    let mut matching_pids: HashSet<usize> = HashSet::new();

    for info in enumerate_handles(&ntdll) {
        if kernel_paths_match(&info.kernel_file_name, &file_kernel_names, &dir_kernel_names) {
            matching_pids.insert(info.pid);
        }
    }

    // Scan loaded modules of all processes.
    let mut all_pids = vec![0u32; 4096];
    let mut cb_needed = 0u32;
    let ok = unsafe {
        EnumProcesses(
            all_pids.as_mut_ptr(),
            (all_pids.len() * std::mem::size_of::<u32>()) as u32,
            &mut cb_needed,
        )
    };
    if ok != 0 {
        let count = cb_needed as usize / std::mem::size_of::<u32>();
        all_pids.truncate(count);
        for &pid in &all_pids {
            if pid == 0 {
                continue;
            }
            for module_path in get_process_modules(pid) {
                if let Some(kn) = wide_path_to_kernel_name(&module_path, &ntdll) {
                    if kernel_paths_match(&kn, &file_kernel_names, &dir_kernel_names) {
                        matching_pids.insert(pid as usize);
                    }
                }
            }
        }
    }

    matching_pids.into_iter().collect()
}
