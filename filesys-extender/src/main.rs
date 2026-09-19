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

use filesys_extender::exec::{execute_steps, steps_for_plan, write_persistence_conf, Step};
use gtk4::{Entry, TextView};

fn build_confirm_page(
    selected_disk: Rc<RefCell<Option<Disk>>>,
    on_apply: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let instructions = Label::new(None);
    let entry = Entry::new();
    let apply_button = Button::with_label("Apply");
    apply_button.set_sensitive(false);

    container.append(&instructions);
    container.append(&entry);
    container.append(&apply_button);

    {
        let selected_disk = selected_disk.clone();
        let instructions = instructions.clone();
        container.connect_map(move |_| {
            if let Some(disk) = selected_disk.borrow().clone() {
                instructions.set_text(&format!(
                    "Type '{}' exactly to confirm this destructive operation:",
                    disk.path
                ));
            }
        });
    }

    {
        let selected_disk = selected_disk.clone();
        let apply_button = apply_button.clone();
        entry.connect_changed(move |entry| {
            let expected = selected_disk.borrow().as_ref().map(|d| d.path.clone());
            let matches = expected.as_deref() == Some(entry.text().as_str());
            apply_button.set_sensitive(matches);
        });
    }

    apply_button.connect_clicked(move |_| on_apply());
    container
}

#[derive(Debug)]
enum ExecMsg {
    StepStarted(String),
    StepOutput(String),
    StepFailed(String),
    Finished(bool),
}

/// Builds the executing page. Returns the page widget plus a start closure
/// the confirm page's Apply button invokes to kick off execution. The
/// spawned worker thread only ever sends owned, `Send` data (`ExecMsg`)
/// over a `std::sync::mpsc` channel - all GTK/state mutation happens back
/// on the main thread inside a `timeout_add_local` poll, since GTK widgets
/// (and `Rc<RefCell<_>>` state) are not `Send` and must never be touched
/// from the worker thread.
fn build_executing_page(
    plan: Rc<RefCell<Option<Plan>>>,
    result_text: Rc<RefCell<String>>,
    stack: Stack,
) -> (GtkBox, Rc<dyn Fn()>) {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let log_view = TextView::new();
    log_view.set_editable(false);
    container.append(&log_view);

    let start: Rc<dyn Fn()> = Rc::new(move || {
        let Some(current_plan) = plan.borrow().clone() else { return };
        let (tx, rx) = std::sync::mpsc::channel::<ExecMsg>();
        let steps = steps_for_plan(&current_plan);

        {
            let buf = log_view.buffer();
            let result_text = result_text.clone();
            let stack = stack.clone();
            gtk4::glib::source::timeout_add_local(std::time::Duration::from_millis(50), move || {
                let mut finished = false;
                for msg in rx.try_iter() {
                    let mut end = buf.end_iter();
                    match &msg {
                        ExecMsg::StepStarted(desc) => buf.insert(&mut end, &format!("==> {desc}\n")),
                        ExecMsg::StepOutput(line) => buf.insert(&mut end, &format!("{line}\n")),
                        ExecMsg::StepFailed(err) => {
                            buf.insert(&mut end, &format!("FAILED: {err}\n"));
                            result_text.borrow_mut().push_str(&format!("FAILED: {err}\n"));
                        }
                        ExecMsg::Finished(success) => {
                            buf.insert(
                                &mut end,
                                if *success { "Done.\n" } else { "Stopped after failure.\n" },
                            );
                            stack.set_visible_child_name("result");
                            finished = true;
                        }
                    }
                }
                if finished {
                    gtk4::glib::ControlFlow::Break
                } else {
                    gtk4::glib::ControlFlow::Continue
                }
            });
        }

        let sender = tx;
        std::thread::spawn(move || {
            let sender_for_cb = sender.clone();
            let outcome = execute_steps(&steps, move |step, res| {
                let _ = sender_for_cb.send(ExecMsg::StepStarted(step.description.clone()));
                match res {
                    Ok(out) => {
                        let _ = sender_for_cb.send(ExecMsg::StepOutput(out.stdout.clone()));
                    }
                    Err(e) => {
                        let _ = sender_for_cb.send(ExecMsg::StepFailed(e.to_string()));
                    }
                }
            });

            if outcome.is_ok() {
                if let Plan::Create { device, partition_number, .. }
                | Plan::Grow { device, partition_number, .. } = &current_plan
                {
                    let mount_dir = format!("/mnt/filesys-extender-{partition_number}");
                    let _ = std::fs::create_dir_all(&mount_dir);
                    let mount_result = execute_steps(
                        &[Step {
                            description: "Mount persistence partition".into(),
                            argv: vec![
                                "mount".into(),
                                format!("{device}{partition_number}"),
                                mount_dir.clone(),
                            ],
                        }],
                        |_, _| {},
                    );
                    if mount_result.is_ok() {
                        let _ = write_persistence_conf(&mount_dir);
                        let _ = execute_steps(
                            &[Step {
                                description: "Unmount".into(),
                                argv: vec!["umount".into(), mount_dir],
                            }],
                            |_, _| {},
                        );
                    }
                }
            }

            let _ = sender.send(ExecMsg::Finished(outcome.is_ok()));
        });
    });

    (container, start)
}

fn build_result_page(result_text: Rc<RefCell<String>>) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let label = Label::new(None);
    container.append(&label);
    container.connect_map(move |_| {
        let text = result_text.borrow();
        if text.is_empty() {
            label.set_text("Completed successfully.");
        } else {
            label.set_text(&text);
        }
    });
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

        let result_text: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

        let (executing_page, start_execution) =
            build_executing_page(plan.clone(), result_text.clone(), stack.clone());
        stack.add_titled(&executing_page, Some("executing"), "Executing");

        let stack_for_confirm_nav = stack.clone();
        let confirm_page = build_confirm_page(selected_disk.clone(), move || {
            stack_for_confirm_nav.set_visible_child_name("executing");
            start_execution();
        });
        stack.add_titled(&confirm_page, Some("confirm"), "Confirm");

        let result_page = build_result_page(result_text.clone());
        stack.add_titled(&result_page, Some("result"), "Result");

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
