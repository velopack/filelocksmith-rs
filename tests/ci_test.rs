use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Sub-mode: hold an exclusive lock on a file and sleep
    if args.len() > 2 && args[1] == "--lock" {
        lock_file(&args[2]);
        return;
    }

    println!("elevated: {}", filelocksmith::is_process_elevated());
    println!("debug_privilege: {}", filelocksmith::set_debug_privilege());

    // Use a directory under the working dir (not temp — avoids CI path quirks)
    let test_dir = std::env::current_dir()
        .expect("no cwd")
        .join("ci-test-workdir");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("failed to create test directory");
    let test_file = test_dir.join("locked.txt");
    fs::write(&test_file, "test content").expect("failed to create test file");

    // Spawn ourselves in --lock mode (instant startup, no PowerShell dependency)
    let self_exe = std::env::current_exe().expect("no current exe");
    let mut child = Command::new(&self_exe)
        .args(["--lock", test_file.to_str().unwrap()])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn locker");

    let child_pid = child.id() as usize;
    println!("spawned locker pid={child_pid} exe={}", self_exe.display());

    // Wait for the locker to signal it acquired the lock
    let mut stdout = BufReader::new(child.stdout.take().expect("no stdout"));
    let mut line = String::new();
    stdout.read_line(&mut line).expect("failed to read from locker");
    assert!(
        line.trim() == "locked",
        "unexpected output from locker: {line:?}"
    );
    println!("locker signalled ready");

    // Detect the lock
    let pids = filelocksmith::find_processes_locking_path(&test_file);
    println!("locking pids: {pids:?}");

    if !pids.contains(&child_pid) {
        // Retry once after a short delay
        thread::sleep(Duration::from_secs(2));
        let pids2 = filelocksmith::find_processes_locking_path(&test_file);
        println!("locking pids (retry): {pids2:?}");

        // Also try the directory
        let dir_pids = filelocksmith::find_processes_locking_path(&test_dir);
        println!("locking pids for dir: {dir_pids:?}");

        assert!(
            pids2.contains(&child_pid) || dir_pids.contains(&child_pid),
            "expected pid {child_pid} in locking pids (file: {pids2:?}, dir: {dir_pids:?})"
        );
    }

    // Verify process path
    let proc_path =
        filelocksmith::pid_to_process_path(child_pid).expect("failed to get process path");
    println!("process path: {proc_path}");

    // Kill the locking processes
    let pids_to_kill = vec![child_pid];
    assert!(
        filelocksmith::quit_processes(pids_to_kill),
        "quit_processes returned false"
    );
    println!("killed locking processes");

    // Wait for the child to actually exit
    let status = child.wait().expect("failed to wait for child");
    println!("child exited: {status}");

    // Confirm the file is no longer locked by our process
    let pids_after = filelocksmith::find_processes_locking_path(&test_file);
    assert!(
        !pids_after.contains(&child_pid),
        "pid {child_pid} still locking after kill"
    );
    println!("verified process no longer locking");

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    println!("all checks passed!");
}

#[cfg(windows)]
fn lock_file(path: &str) {
    use std::os::windows::fs::OpenOptionsExt;

    let _file = fs::OpenOptions::new()
        .read(true)
        .share_mode(0) // exclusive — no sharing
        .open(path)
        .expect("failed to open file for locking");

    // Signal to the parent that the lock is held
    println!("locked");

    // Keep the handle open indefinitely
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(not(windows))]
fn lock_file(_path: &str) {
    panic!("this test only works on Windows");
}
