use std::os::unix::process::CommandExt;
use std::process::Command;

fn ensure_root() {
    // SAFETY: geteuid() is a pure syscall with no preconditions.
    let euid = unsafe { libc::geteuid() };
    if euid == 0 {
        return;
    }
    let self_path = std::env::current_exe().expect("cannot resolve own executable path");
    let err = Command::new("pkexec").arg(self_path).exec();
    // exec() only returns on failure.
    eprintln!("failed to re-exec via pkexec: {err}");
    std::process::exit(1);
}

fn main() {
    ensure_root();
    run_app();
}

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Label, Stack};

fn run_app() {
    let app = Application::builder()
        .application_id("dev.dreamos.filesys-extender")
        .build();

    app.connect_activate(|app| {
        let stack = Stack::new();
        stack.add_titled(&Label::new(Some("Disk list (Task 8)")), Some("disk_list"), "Disk list");
        stack.add_titled(&Label::new(Some("Inspect (Task 9)")), Some("inspect"), "Inspect");
        stack.add_titled(&Label::new(Some("Confirm (Task 10)")), Some("confirm"), "Confirm");
        stack.add_titled(&Label::new(Some("Executing (Task 10)")), Some("executing"), "Executing");
        stack.add_titled(&Label::new(Some("Result (Task 10)")), Some("result"), "Result");
        stack.set_visible_child_name("disk_list");

        let window = ApplicationWindow::builder()
            .application(app)
            .title("dreamos - filesys-extender")
            .default_width(700)
            .default_height(500)
            .child(&stack)
            .build();
        window.present();
    });

    app.run();
}
