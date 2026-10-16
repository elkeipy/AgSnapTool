#![windows_subsystem = "windows"]

fn main() {
    let code = agsnap::run();
    if code != 0 {
        std::process::exit(code);
    }
}
