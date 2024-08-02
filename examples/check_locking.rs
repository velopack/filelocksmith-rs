fn main() {
    println!("{:?}", file_locksmith::is_process_elevated());
    println!("{:?}", file_locksmith::set_debug_privilege());
    println!("{:?}", file_locksmith::pid_to_process_path(12352));
}