use std::os::unix::process::CommandExt;
use std::process::Command;

fn ensure_root() {
    // Escape hatch for UI-only iteration (see scripts/run-nvidia-installer.sh):
    // skips the pkexec re-exec so the wizard's screens can be exercised
    // without a polkit prompt each run. apt/mokutil steps still fail
    // without real root, as expected.
    if std::env::var_os("NVIDIA_INSTALLER_SKIP_ROOT").is_some() {
        return;
    }
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
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, Entry, Image, Label,
    Orientation, PasswordEntry, Stack, TextView,
};
use nvidia_installer::detect::{detect_gpus, CurrentDriver, GpuDevice};
use nvidia_installer::exec::{apt_install_argv, apt_update_argv, run_cmd, run_cmd_with_stdin};
use nvidia_installer::recommend::{recommend_package, Recommendation, RecommendationSource};
use nvidia_installer::secureboot::{
    ensure_mok_keypair, is_key_enrolled, mokutil_import_argv, secure_boot_state, SecureBootState,
};
use nvidia_installer::state::{clear_state, load_state, save_state, State};
use std::cell::RefCell;
use std::rc::Rc;

const LOGO_BYTES: &[u8] = include_bytes!("../assets/ball.png");

fn spacer() -> GtkBox {
    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    spacer
}

fn driver_label(driver: &CurrentDriver) -> &'static str {
    match driver {
        CurrentDriver::Nouveau => "nouveau (open source)",
        CurrentDriver::Nvidia => "nvidia (proprietary, already installed)",
        CurrentDriver::None => "none bound",
    }
}

fn build_welcome_page(on_next: impl Fn() + 'static) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 12);
    container.set_margin_top(16);
    container.set_margin_bottom(16);
    container.set_margin_start(16);
    container.set_margin_end(16);

    let header = GtkBox::new(Orientation::Horizontal, 12);
    let logo_bytes = gtk4::glib::Bytes::from_static(LOGO_BYTES);
    let logo_texture = gtk4::gdk::Texture::from_bytes(&logo_bytes)
        .expect("assets/ball.png is a valid, embedded PNG");
    let logo = Image::from_paintable(Some(&logo_texture));
    logo.set_pixel_size(64);
    let title = Label::new(None);
    title.set_markup("<span size='xx-large' weight='bold'>DreamOS NVIDIA Driver Installer</span>");
    header.append(&logo);
    header.append(&title);
    container.append(&header);

    let intro = Label::new(Some(
        "This wizard detects NVIDIA GPUs and installs the proprietary \
         driver via apt.\n\n\
         How it works:\n\
         1. Detect - find NVIDIA GPU(s) and the driver currently bound.\n\
         2. Recommend - the correct driver package for your card.\n\
         3. Secure Boot - if enabled, sets up DKMS module signing so the \
         driver actually loads after reboot (a Secure Boot system \
         otherwise silently rejects unsigned third-party kernel modules).\n\
         4. Confirm - review the exact commands before anything runs.\n\
         5. Install - watch the apt install run, with live output.\n\n\
         This tool never edits your apt sources and only installs the \
         one package you confirm.",
    ));
    intro.set_wrap(true);
    intro.set_justify(gtk4::Justification::Left);
    intro.set_halign(gtk4::Align::Start);
    container.append(&intro);
    container.append(&spacer());

    let next_button = Button::with_label("Get Started");
    next_button.set_halign(gtk4::Align::End);
    next_button.connect_clicked(move |_| on_next());
    container.append(&next_button);

    container
}

fn build_detect_page(
    gpus: Rc<RefCell<Vec<GpuDevice>>>,
    on_next: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let summary = Label::new(None);
    summary.set_wrap(true);
    summary.set_halign(gtk4::Align::Start);
    summary.set_valign(gtk4::Align::Start);
    let next_button = Button::with_label("Next");
    next_button.set_sensitive(false);

    container.append(&summary);
    container.append(&spacer());
    container.append(&next_button);

    {
        let gpus = gpus.clone();
        let summary = summary.clone();
        let next_button = next_button.clone();
        container.connect_map(move |_| {
            let detected = detect_gpus().unwrap_or_default();
            if detected.is_empty() {
                summary.set_text("No NVIDIA GPU detected on this system. Nothing to do.");
                next_button.set_sensitive(false);
            } else {
                let lines: Vec<String> = detected
                    .iter()
                    .map(|g| format!("{}: {} (driver: {})", g.pci_slot, g.model, driver_label(&g.driver)))
                    .collect();
                summary.set_text(&lines.join("\n"));
                next_button.set_sensitive(true);
            }
            *gpus.borrow_mut() = detected;
        });
    }

    next_button.connect_clicked(move |_| on_next());
    container
}

fn build_recommend_page(
    chosen_package: Rc<RefCell<Option<String>>>,
    on_next: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let source_label = Label::new(None);
    source_label.set_halign(gtk4::Align::Start);
    let entry = Entry::new();
    let next_button = Button::with_label("Next");

    container.append(&source_label);
    container.append(&entry);
    container.append(&spacer());
    container.append(&next_button);

    {
        let chosen_package = chosen_package.clone();
        let source_label = source_label.clone();
        let entry = entry.clone();
        container.connect_map(move |_| {
            let Recommendation { package, source } = recommend_package();
            source_label.set_text(match source {
                RecommendationSource::Detected => "Detected via nvidia-detect:",
                RecommendationSource::FallbackDefault => {
                    "nvidia-detect unavailable - best guess (edit if you know your card needs a different package):"
                }
            });
            entry.set_text(&package);
            *chosen_package.borrow_mut() = Some(package);
        });
    }

    {
        let chosen_package = chosen_package.clone();
        entry.connect_changed(move |entry| {
            *chosen_package.borrow_mut() = Some(entry.text().to_string());
        });
    }

    next_button.connect_clicked(move |_| on_next());
    container
}

enum SecureBootOutcome {
    NotApplicable,
    OptedOut,
    Enrolled,
    Failed(String),
}

fn build_secureboot_page(
    outcome: Rc<RefCell<SecureBootOutcome>>,
    on_next: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let status_label = Label::new(None);
    status_label.set_wrap(true);
    status_label.set_halign(gtk4::Align::Start);
    let opt_out = CheckButton::with_label("I'll handle Secure Boot / module signing myself");
    let pass1 = PasswordEntry::new();
    pass1.set_show_peek_icon(true);
    let pass2 = PasswordEntry::new();
    pass2.set_show_peek_icon(true);
    let enroll_button = Button::with_label("Generate key and enroll");
    let error_label = Label::new(None);
    error_label.set_halign(gtk4::Align::Start);
    let next_button = Button::with_label("Next");

    container.append(&status_label);
    container.append(&opt_out);
    container.append(&pass1);
    container.append(&pass2);
    container.append(&enroll_button);
    container.append(&error_label);
    container.append(&spacer());
    container.append(&next_button);

    {
        let status_label = status_label.clone();
        let opt_out = opt_out.clone();
        let pass1 = pass1.clone();
        let pass2 = pass2.clone();
        let enroll_button = enroll_button.clone();
        let next_button = next_button.clone();
        let outcome = outcome.clone();
        container.connect_map(move |_| {
            match secure_boot_state() {
                SecureBootState::Disabled => {
                    status_label.set_text("Secure Boot is off - no module signing needed.");
                    opt_out.set_visible(false);
                    pass1.set_visible(false);
                    pass2.set_visible(false);
                    enroll_button.set_visible(false);
                    *outcome.borrow_mut() = SecureBootOutcome::NotApplicable;
                    next_button.set_sensitive(true);
                }
                SecureBootState::Enabled | SecureBootState::Unknown => {
                    status_label.set_text(
                        "Secure Boot is on. Without an enrolled signing key, the \
                         driver's kernel module will build but fail to load after \
                         reboot. Enter a one-time enrollment password (min 8 chars, \
                         shown twice) to generate and enroll a signing key, or opt \
                         out and handle it yourself.",
                    );
                    next_button.set_sensitive(false);
                }
            }
        });
    }

    {
        let next_button = next_button.clone();
        let pass1 = pass1.clone();
        let pass2 = pass2.clone();
        let enroll_button = enroll_button.clone();
        opt_out.connect_toggled(move |btn| {
            let opted_out = btn.is_active();
            pass1.set_sensitive(!opted_out);
            pass2.set_sensitive(!opted_out);
            enroll_button.set_sensitive(!opted_out);
            next_button.set_sensitive(opted_out);
        });
    }

    {
        let outcome = outcome.clone();
        opt_out.connect_toggled(move |btn| {
            if btn.is_active() {
                *outcome.borrow_mut() = SecureBootOutcome::OptedOut;
            }
        });
    }

    {
        let pass1 = pass1.clone();
        let pass2 = pass2.clone();
        let error_label = error_label.clone();
        let next_button = next_button.clone();
        let outcome = outcome.clone();
        enroll_button.connect_clicked(move |_| {
            let p1 = pass1.text().to_string();
            let p2 = pass2.text().to_string();
            if p1.len() < 8 {
                error_label.set_text("Password must be at least 8 characters.");
                return;
            }
            if p1 != p2 {
                error_label.set_text("Passwords don't match.");
                return;
            }
            if let Err(e) = ensure_mok_keypair() {
                error_label.set_text(&format!("Key generation failed: {e}"));
                *outcome.borrow_mut() = SecureBootOutcome::Failed(e.to_string());
                return;
            }
            let argv = mokutil_import_argv();
            let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
            // mokutil --import reads the password twice from stdin.
            let stdin_data = format!("{p1}\n{p1}\n");
            match run_cmd_with_stdin(&argv_refs, &stdin_data) {
                Ok(_) => {
                    error_label.set_text("Key enrolled - will take effect after reboot.");
                    *outcome.borrow_mut() = SecureBootOutcome::Enrolled;
                    next_button.set_sensitive(true);
                }
                Err(e) => {
                    error_label.set_text(&format!("Enrollment failed: {e}"));
                    *outcome.borrow_mut() = SecureBootOutcome::Failed(e.to_string());
                }
            }
        });
    }

    next_button.connect_clicked(move |_| on_next());
    container
}

fn build_reboot_required_page() -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let label = Label::new(Some(
        "A signing key was generated and an enrollment request submitted.\n\n\
         Reboot now. On the blue 'MOK Management' screen, choose 'Enroll \
         MOK', then 'Continue', and enter the same password you just typed \
         to confirm enrollment.\n\n\
         After rebooting, re-run this tool to continue installing the driver.",
    ));
    label.set_wrap(true);
    label.set_halign(gtk4::Align::Start);
    container.append(&label);
    container
}

fn build_confirm_page(
    chosen_package: Rc<RefCell<Option<String>>>,
    outcome: Rc<RefCell<SecureBootOutcome>>,
    on_apply: impl Fn() + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let summary = Label::new(None);
    summary.set_wrap(true);
    summary.set_halign(gtk4::Align::Start);
    let apply_button = Button::with_label("Apply");

    container.append(&summary);
    container.append(&spacer());
    container.append(&apply_button);

    {
        let chosen_package = chosen_package.clone();
        let outcome = outcome.clone();
        let summary = summary.clone();
        container.connect_map(move |_| {
            let package = chosen_package.borrow().clone().unwrap_or_default();
            let sb_line = match &*outcome.borrow() {
                SecureBootOutcome::NotApplicable => "Secure Boot: off, no signing needed.".to_string(),
                SecureBootOutcome::OptedOut => "Secure Boot: on, signing skipped at your request.".to_string(),
                SecureBootOutcome::Enrolled => "Secure Boot: key enrolled this run.".to_string(),
                SecureBootOutcome::Failed(err) => {
                    format!("Secure Boot: enrollment failed ({err}), proceeding without it.")
                }
            };
            summary.set_text(&format!(
                "About to run:\n  apt-get update\n  apt-get install -y {package}\n\n{sb_line}"
            ));
        });
    }

    apply_button.connect_clicked(move |_| on_apply());
    container
}

#[derive(Debug)]
enum ExecMsg {
    Line(String),
    Failed(String),
    Finished(bool),
}

fn build_executing_page(
    chosen_package: Rc<RefCell<Option<String>>>,
    result_text: Rc<RefCell<String>>,
    stack: Stack,
) -> (GtkBox, Rc<dyn Fn()>) {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let log_view = TextView::new();
    log_view.set_editable(false);
    container.append(&log_view);

    let start: Rc<dyn Fn()> = Rc::new(move || {
        let Some(package) = chosen_package.borrow().clone() else { return };
        let (tx, rx) = std::sync::mpsc::channel::<ExecMsg>();

        {
            let buf = log_view.buffer();
            let result_text = result_text.clone();
            let stack = stack.clone();
            gtk4::glib::source::timeout_add_local(std::time::Duration::from_millis(50), move || {
                let mut finished = false;
                for msg in rx.try_iter() {
                    let mut end = buf.end_iter();
                    match &msg {
                        ExecMsg::Line(line) => buf.insert(&mut end, &format!("{line}\n")),
                        ExecMsg::Failed(err) => {
                            buf.insert(&mut end, &format!("FAILED: {err}\n"));
                            result_text.borrow_mut().push_str(&format!("FAILED: {err}\n"));
                        }
                        ExecMsg::Finished(success) => {
                            buf.insert(&mut end, if *success { "Done.\n" } else { "Stopped after failure.\n" });
                            stack.set_visible_child_name("result");
                            finished = true;
                        }
                    }
                }
                if finished { gtk4::glib::ControlFlow::Break } else { gtk4::glib::ControlFlow::Continue }
            });
        }

        std::thread::spawn(move || {
            let update_owned = apt_update_argv();
            let update_argv: Vec<&str> = update_owned.iter().map(String::as_str).collect();
            let _ = tx.send(ExecMsg::Line("==> apt-get update".into()));
            let update_result = run_cmd(&update_argv);
            match update_result {
                Ok(out) => {
                    let _ = tx.send(ExecMsg::Line(out.stdout));
                }
                Err(e) => {
                    let _ = tx.send(ExecMsg::Failed(e.to_string()));
                    let _ = tx.send(ExecMsg::Finished(false));
                    return;
                }
            }

            let install_owned = apt_install_argv(&package);
            let install_argv: Vec<&str> = install_owned.iter().map(String::as_str).collect();
            let _ = tx.send(ExecMsg::Line(format!("==> apt-get install -y {package}")));
            let success = match run_cmd(&install_argv) {
                Ok(out) => {
                    let _ = tx.send(ExecMsg::Line(out.stdout));
                    true
                }
                Err(e) => {
                    let _ = tx.send(ExecMsg::Failed(e.to_string()));
                    false
                }
            };
            let _ = tx.send(ExecMsg::Finished(success));
        });
    });

    (container, start)
}

fn build_result_page(result_text: Rc<RefCell<String>>) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let label = Label::new(None);
    label.set_wrap(true);
    container.append(&label);
    container.connect_map(move |_| {
        let text = result_text.borrow();
        label.set_text(if text.is_empty() {
            "Driver installed. Reboot to load it (or, if Secure Boot enrollment \
             happened this run, reboot and complete MOK enrollment first)."
        } else {
            &text
        });
    });
    container
}

fn build_verify_page(on_continue: impl Fn() + 'static, on_done: impl Fn() + 'static) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 8);
    let label = Label::new(None);
    label.set_wrap(true);
    label.set_halign(gtk4::Align::Start);
    container.append(&label);
    container.append(&spacer());

    let continue_button = Button::with_label("Continue to driver install");
    let done_button = Button::with_label("Done");
    container.append(&continue_button);
    container.append(&done_button);

    {
        let label = label.clone();
        let continue_button = continue_button.clone();
        let done_button = done_button.clone();
        container.connect_map(move |_| {
            let enrolled = run_cmd(&["mokutil", "--list-enrolled"])
                .map(|out| is_key_enrolled(&out.stdout))
                .unwrap_or(false);
            let module_loaded = run_cmd(&["lsmod"])
                .map(|out| out.stdout.lines().any(|l| l.starts_with("nvidia ")))
                .unwrap_or(false);

            let mok_line = if enrolled { "MOK key: enrolled." } else { "MOK key: still not enrolled - did you complete the blue MokManager screen?" };
            let module_line = if module_loaded { "nvidia module: loaded." } else { "nvidia module: not loaded yet." };
            label.set_text(&format!("{mok_line}\n{module_line}"));

            if enrolled {
                let _ = clear_state();
            }
            continue_button.set_visible(enrolled && !module_loaded);
            done_button.set_visible(enrolled && module_loaded);
        });
    }

    continue_button.connect_clicked(move |_| on_continue());
    done_button.connect_clicked(move |_| on_done());
    container
}

fn run_app() {
    let app = Application::builder()
        .application_id("dev.dreamos.nvidia-installer")
        .build();

    app.connect_activate(|app| {
        let stack = Stack::new();
        let gpus: Rc<RefCell<Vec<GpuDevice>>> = Rc::new(RefCell::new(Vec::new()));
        let chosen_package: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let sb_outcome = Rc::new(RefCell::new(SecureBootOutcome::NotApplicable));
        let result_text: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

        if load_state().mok_enrollment_pending {
            let verify_page = build_verify_page(
                {
                    let stack = stack.clone();
                    move || stack.set_visible_child_name("confirm")
                },
                {
                    let app = app.clone();
                    move || app.quit()
                },
            );
            stack.add_named(&verify_page, Some("verify"));

            let (executing_page, start_exec) =
                build_executing_page(chosen_package.clone(), result_text.clone(), stack.clone());
            let start_exec_for_confirm = start_exec.clone();
            let confirm_page = build_confirm_page(chosen_package.clone(), sb_outcome.clone(), {
                let stack = stack.clone();
                move || {
                    start_exec_for_confirm();
                    stack.set_visible_child_name("executing");
                }
            });
            stack.add_named(&confirm_page, Some("confirm"));
            stack.add_named(&executing_page, Some("executing"));

            let result_page = build_result_page(result_text.clone());
            stack.add_named(&result_page, Some("result"));

            stack.set_visible_child_name("verify");
        } else {
            let welcome = build_welcome_page({
                let stack = stack.clone();
                move || stack.set_visible_child_name("detect")
            });
            stack.add_named(&welcome, Some("welcome"));

            let detect_page = build_detect_page(gpus.clone(), {
                let stack = stack.clone();
                move || stack.set_visible_child_name("recommend")
            });
            stack.add_named(&detect_page, Some("detect"));

            let recommend_page = build_recommend_page(chosen_package.clone(), {
                let stack = stack.clone();
                move || stack.set_visible_child_name("secureboot")
            });
            stack.add_named(&recommend_page, Some("recommend"));

            let secureboot_page = build_secureboot_page(sb_outcome.clone(), {
                let stack = stack.clone();
                let sb_outcome = sb_outcome.clone();
                move || {
                    if matches!(&*sb_outcome.borrow(), SecureBootOutcome::Enrolled) {
                        let _ = save_state(&State { mok_enrollment_pending: true });
                        stack.set_visible_child_name("reboot_required");
                    } else {
                        stack.set_visible_child_name("confirm");
                    }
                }
            });
            stack.add_named(&secureboot_page, Some("secureboot"));

            let reboot_page = build_reboot_required_page();
            stack.add_named(&reboot_page, Some("reboot_required"));

            let (executing_page, start_exec) =
                build_executing_page(chosen_package.clone(), result_text.clone(), stack.clone());
            let start_exec_for_confirm = start_exec.clone();
            let confirm_page = build_confirm_page(chosen_package.clone(), sb_outcome.clone(), {
                let stack = stack.clone();
                move || {
                    start_exec_for_confirm();
                    stack.set_visible_child_name("executing");
                }
            });
            stack.add_named(&confirm_page, Some("confirm"));
            stack.add_named(&executing_page, Some("executing"));

            let result_page = build_result_page(result_text.clone());
            stack.add_named(&result_page, Some("result"));

            stack.set_visible_child_name("welcome");
        }

        let window = ApplicationWindow::builder()
            .application(app)
            .title("DreamOS NVIDIA Driver Installer")
            .default_width(640)
            .default_height(480)
            .child(&stack)
            .build();
        window.present();
    });

    app.run();
}
