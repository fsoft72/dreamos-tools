mod render;
mod ui;

use dreamos_install::catalog::{resolve_root, ROOT_ENV};
use gtk4::prelude::*;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const APP_ID: &str = "org.dreamos.Install";

/// Re-executes the binary via pkexec unless already root, forwarding the
/// catalog root as an argument (pkexec drops the environment).
fn ensure_root(root: &Path) {
    // Escape hatch for UI-only iteration (see scripts/run-dreamos-install.sh):
    // skips the pkexec re-exec so the browser can be exercised without a
    // polkit prompt each run. Scripts that need root fail, as expected.
    if std::env::var_os("DREAMOS_INSTALL_SKIP_ROOT").is_some() {
        return;
    }
    // SAFETY: geteuid() is a pure syscall with no preconditions.
    let euid = unsafe { libc::geteuid() };
    if euid == 0 {
        return;
    }
    let self_path = std::env::current_exe().expect("cannot resolve own executable path");
    let err = Command::new("pkexec").arg(self_path).arg(root).exec();
    // exec() only returns on failure.
    eprintln!("failed to re-exec via pkexec: {err}");
    std::process::exit(1);
}

/// Resolves the catalog root and makes it absolute, so it survives the
/// pkexec re-exec (which changes the working directory).
fn catalog_root() -> PathBuf {
    let root = resolve_root(std::env::args_os().nth(1), std::env::var_os(ROOT_ENV));
    std::path::absolute(&root).unwrap_or(root)
}

fn main() -> gtk4::glib::ExitCode {
    let root = catalog_root();
    ensure_root(&root);

    let app = gtk4::Application::builder()
        .application_id(APP_ID)
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(move |app| {
        let scan = ui::load_catalog(&root);
        ui::build_window(app, &root, scan).present();
    });
    // GTK must not parse our CLI argument (the catalog root).
    app.run_with_args::<&str>(&[])
}
