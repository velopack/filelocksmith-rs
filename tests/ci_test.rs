use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let test_dir = std::env::temp_dir().join("filelocksmith-ci-test");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("failed to create test directory");
    let test_file = test_dir.join("locked.txt");
    fs::write(&test_file, "test content").expect("failed to create test file");

    let test_file_str = test_file.to_str().expect("invalid path");

    // Spawn a PowerShell process that holds an exclusive file lock
    let ps_command = format!(
        "$f = [System.IO.File]::Open('{}', 'Open', 'Read', 'None'); Start-Sleep -Seconds 300",
        test_file_str
    );

    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &ps_command])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn powershell");

    let child_pid = child.id() as usize;
    println!("spawned locking process pid={child_pid}");

    // Poll until the lock is detected (PowerShell can be slow to start on CI)
    let timeout = Duration::from_secs(30);
    let start = Instant::now();
    let mut pids;
    loop {
        thread::sleep(Duration::from_secs(1));
        pids = filelocksmith::find_processes_locking_path(&test_file);
        if pids.contains(&child_pid) {
            break;
        }
        if start.elapsed() > timeout {
            // Check if PowerShell crashed
            if let Some(status) = child.try_wait().expect("failed to check child") {
                let mut stderr_buf = String::new();
                if let Some(mut stderr) = child.stderr.take() {
                    std::io::Read::read_to_string(&mut stderr, &mut stderr_buf).ok();
                }
                panic!("powershell exited early with {status}, locking pids: {pids:?}, stderr: {stderr_buf}");
            }
            panic!("timed out waiting for pid {child_pid} in locking pids {pids:?}");
        }
    }
    println!("locking pids: {pids:?} (detected after {:?})", start.elapsed());

    // Verify we can resolve the process path
    let proc_path =
        filelocksmith::pid_to_process_path(child_pid).expect("failed to get process path");
    println!("process path: {proc_path}");
    assert!(
        proc_path.to_lowercase().contains("powershell"),
        "expected path to contain 'powershell', got: {proc_path}"
    );

    // Kill the locking processes
    assert!(
        filelocksmith::quit_processes(pids),
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
