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

use filesys_extender::disk::{detect_live_boot_disk, list_candidates, Disk};
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, Label, ListBox, ListBoxRow,
    Orientation, Stack,
};
use std::cell::RefCell;
use std::rc::Rc;

fn format_size(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    format!("{:.1} GiB", bytes as f64 / GIB)
}

fn build_disk_list_page(
    selected_disk: Rc<RefCell<Option<Disk>>>,
    on_next: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let list = ListBox::new();

    let disks = list_candidates().unwrap_or_default();
    let live_boot = detect_live_boot_disk();

    let disks_rc = Rc::new(disks);
    for (idx, disk) in disks_rc.iter().enumerate() {
        let label_text = format!(
            "{}  -  {}  -  {}",
            disk.path,
            format_size(disk.size_bytes),
            disk.model
        );
        let row = ListBoxRow::new();
        row.set_child(Some(&gtk4::Label::new(Some(&label_text))));
        row.set_widget_name(&idx.to_string());
        if Some(disk.path.clone()) == live_boot {
            list.select_row(Some(&row));
        }
        list.append(&row);
    }

    let next_button = Button::with_label("Next");
    next_button.set_sensitive(false);

    {
        let disks_rc = disks_rc.clone();
        let selected_disk = selected_disk.clone();
        let next_button_clone = next_button.clone();
        list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let idx: usize = row.widget_name().parse().unwrap_or(usize::MAX);
                if let Some(disk) = disks_rc.get(idx) {
                    *selected_disk.borrow_mut() = Some(disk.clone());
                    next_button_clone.set_sensitive(true);
                }
            }
        });
    }

    next_button.connect_clicked(move |_| on_next());

    container.append(&list);
    container.append(&next_button);
    container
}

fn run_app() {
    let app = Application::builder()
        .application_id("dev.dreamos.filesys-extender")
        .build();

    app.connect_activate(|app| {
        let stack = Stack::new();

        let selected_disk: Rc<RefCell<Option<Disk>>> = Rc::new(RefCell::new(None));
        let stack_for_nav = stack.clone();
        let disk_list_page = build_disk_list_page(selected_disk.clone(), move || {
            stack_for_nav.set_visible_child_name("inspect");
        });
        stack.add_titled(&disk_list_page, Some("disk_list"), "Disk list");

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
