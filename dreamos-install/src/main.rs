mod render;
mod ui;

use dreamos_install::catalog::{resolve_root, ROOT_ENV};
use gtk4::prelude::*;
use std::path::PathBuf;

const APP_ID: &str = "org.dreamos.Install";

/// Resolves the catalog root and makes it absolute.
fn catalog_root() -> PathBuf {
    let root = resolve_root(std::env::args_os().nth(1), std::env::var_os(ROOT_ENV));
    std::path::absolute(&root).unwrap_or(root)
}

fn main() -> gtk4::glib::ExitCode {
    let root = catalog_root();

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
