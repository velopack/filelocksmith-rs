fn main() {
    println!("{:?}", filelocksmith::is_process_elevated());
    println!("{:?}", filelocksmith::set_debug_privilege());
    println!("{:?}", filelocksmith::pid_to_process_path(12352));
    let pids = filelocksmith::find_processes_locking_path(r"C:\Users\Caelan\AppData\Local\AvaloniaCrossPlat");
    println!("{:?}", pids);

    for pid in &pids {
        println!("[{}] {:?}", pid, filelocksmith::pid_to_process_path(*pid));
    }

    if filelocksmith::quit_processes(pids) {
        println!("Processes quit successfully");
    }
}