# filelocksmith-rs
[![Version](https://img.shields.io/crates/v/filelocksmith?style=flat-square)](https://crates.io/crates/filelocksmith)
[![License](https://img.shields.io/crates/l/filelocksmith?style=flat-square)](https://github.com/velopack/filelocksmith-rs/blob/master/LICENSE)

Reliably find and quit processes that are locking a file or folder on Windows.
This is a difficult problem to solve on Windows, as the OS does not provide a built-in or
straight-forward way to do this. Additionally, unlike *nix, files and folders can not be
deleted or moved while they are locked by a process.

This library wraps the FileLocksmith module from the PowerToys project, which is written in C++.
The implementation in PowerToys is endorsed by Microsoft and is very robust.

## Installing
```toml
[dependencies]
filelocksmith = "0.1"
```

## Usage
```rust
use filelocksmith::{find_processes_locking_path, quit_processes, pid_to_process_path};

let path = "C:\\path\\to\\file.txt";
let pids = find_processes_locking_path(path);

// print paths of processes locking the file
for pid in &pids {
   println!("[{}] {:?}", pid, pid_to_process_path(*pid));
}

// quit the processes locking the file
if quit_processes(pids) {
 println!("Processes quit successfully");
}
```
