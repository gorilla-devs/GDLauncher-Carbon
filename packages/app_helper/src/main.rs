use std::thread;

fn main() {
    // Execute external program
    let output = std::process::Command::new("./GDLauncher Carbon.app")
        .output()
        .expect("failed to execute process");
}
