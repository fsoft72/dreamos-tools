mod render;
mod ui;

use dreamos_install::catalog::{resolve_root, ROOT_ENV};
use dreamos_install::elevate::{pkexec_argv, CURSOR_ENV, DARK_ENV, FONT_ENV, ICONS_ENV, THEME_ENV};
use gtk4::prelude::*;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const APP_ID: &str = "org.dreamos.Install";

/// Exports the user's GTK look (theme, dark preference, icons, font, cursor)
/// as environment variables so `ensure_root` forwards them: root has its own
/// default settings and would otherwise ignore the user's dark / light mode.
fn export_user_look() {
    if gtk4::init().is_err() {
        return;
    }
    let Some(settings) = gtk4::Settings::default() else {
        return;
    };
    let names = [
        (THEME_ENV, settings.gtk_theme_name()),
        (ICONS_ENV, settings.gtk_icon_theme_name()),
        (FONT_ENV, settings.gtk_font_name()),
        (CURSOR_ENV, settings.gtk_cursor_theme_name()),
    ];
    for (var, value) in names {
        if let Some(value) = value {
            std::env::set_var(var, value.as_str());
        }
    }
    let dark = settings.is_gtk_application_prefer_dark_theme();
    std::env::set_var(DARK_ENV, if dark { "1" } else { "0" });
}

/// Applies the look forwarded by `export_user_look`; does nothing when the
/// variables are absent (not elevated, or SKIP_ROOT).
fn apply_forwarded_look() {
    let Some(settings) = gtk4::Settings::default() else {
        return;
    };
    if let Ok(theme) = std::env::var(THEME_ENV) {
        settings.set_gtk_theme_name(Some(&theme));
    }
    if let Ok(icons) = std::env::var(ICONS_ENV) {
        settings.set_gtk_icon_theme_name(Some(&icons));
    }
    if let Ok(font) = std::env::var(FONT_ENV) {
        settings.set_gtk_font_name(Some(&font));
    }
    if let Ok(cursor) = std::env::var(CURSOR_ENV) {
        settings.set_gtk_cursor_theme_name(Some(&cursor));
    }
    if let Ok(dark) = std::env::var(DARK_ENV) {
        settings.set_gtk_application_prefer_dark_theme(dark == "1");
    }
}

/// Re-executes the binary via pkexec unless already root, forwarding the
/// catalog root as an argument and the display session variables (pkexec
/// drops the environment).
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
    // pkexec drops the environment, including the variables that point to the
    // display: forward them or GTK fails with "Failed to open display".
    export_user_look();
    let argv = pkexec_argv(&self_path, root, |name| std::env::var_os(name));
    let err = Command::new("pkexec").args(argv).exec();
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
        apply_forwarded_look();
        let scan = ui::load_catalog(&root);
        ui::build_window(app, &root, scan).present();
    });
    // GTK must not parse our CLI argument (the catalog root).
    app.run_with_args::<&str>(&[])
}
