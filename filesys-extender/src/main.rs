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

use filesys_extender::disk::{compute_plan, inspect_free_space, Plan};

fn describe_plan(plan: &Plan) -> String {
    match plan {
        Plan::Create { device, start_bytes, end_bytes, .. } => format!(
            "Will create a new ext4 'persistence' partition on {device}, using {} of free space.",
            format_size(end_bytes - start_bytes)
        ),
        Plan::Grow { device, new_end_bytes, .. } => format!(
            "Will grow the existing 'persistence' partition on {device} up to {}.",
            format_size(*new_end_bytes)
        ),
        Plan::NoAction { reason } => format!("Nothing to do: {reason}"),
    }
}

fn build_inspect_page(
    selected_disk: Rc<RefCell<Option<Disk>>>,
    plan: Rc<RefCell<Option<Plan>>>,
    on_next: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let summary_label = Label::new(None);
    let next_button = Button::with_label("Next");
    next_button.set_sensitive(false);

    container.append(&summary_label);
    container.append(&next_button);

    {
        let selected_disk = selected_disk.clone();
        let plan = plan.clone();
        let summary_label = summary_label.clone();
        let next_button = next_button.clone();
        container.connect_map(move |_| {
            let Some(disk) = selected_disk.borrow().clone() else {
                summary_label.set_text("No disk selected.");
                return;
            };
            let entries = match inspect_free_space(&disk.path) {
                Ok(entries) => entries,
                Err(e) => {
                    summary_label.set_text(&format!("Failed to inspect {}: {e}", disk.path));
                    return;
                }
            };
            let computed = compute_plan(&disk.path, &entries, &disk.partitions);
            summary_label.set_text(&describe_plan(&computed));
            next_button.set_sensitive(!matches!(computed, Plan::NoAction { .. }));
            *plan.borrow_mut() = Some(computed);
        });
    }

    next_button.connect_clicked(move |_| on_next());
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

        let plan: Rc<RefCell<Option<Plan>>> = Rc::new(RefCell::new(None));
        let stack_for_inspect_nav = stack.clone();
        let inspect_page = build_inspect_page(selected_disk.clone(), plan.clone(), move || {
            stack_for_inspect_nav.set_visible_child_name("confirm");
        });
        stack.add_titled(&inspect_page, Some("inspect"), "Inspect");

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
