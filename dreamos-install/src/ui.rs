use crate::render::render_document;
use dreamos_install::catalog::{self, App, AppPos, Category, ScanResult};
use dreamos_install::elevate::{script_argv, SKIP_ROOT_ENV};
use dreamos_install::markdown::{self, escape_markup};
use dreamos_install::status::{invoking_user, run_check, Status, APP_DIR_ENV, CHECK_TIMEOUT, USER_ENV};
use gtk4::glib::{self, ControlFlow, Propagation};
use gtk4::prelude::*;
use gtk4::{
    gio, AlertDialog, Align, Application, ApplicationWindow, Box as GtkBox, Button, Image, Label, ListBox,
    ListBoxRow, Orientation, PolicyType, ScrolledWindow, SearchEntry, SelectionMode, Separator, Stack,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;
use sourceview5::prelude::*;
use vte4::prelude::*;

const LOGO_BYTES: &[u8] = include_bytes!("../assets/ball.png");
const SEARCH_WIDTH_CHARS: i32 = 28;
const APP_ICON_SIZE: i32 = 40;
const PAGE_PLACEHOLDER: &str = "placeholder";
const PAGE_DETAIL: &str = "detail";
const PAGE_TERMINAL: &str = "terminal";
const SCRIPT_WINDOW_WIDTH: i32 = 860;
const SCRIPT_WINDOW_HEIGHT: i32 = 620;
const SCHEME_LIGHT: &str = "Adwaita";
const SCHEME_DARK: &str = "Adwaita-dark";
const CHECK_POLL: Duration = Duration::from_millis(50);
const SCROLLBACK_LINES: i64 = 10_000;

/// A script action the user can run on an app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Install,
    Uninstall,
}

impl Action {
    /// Script file of `app` run for this action (path relative to the
    /// app folder). None for an uninstall the app does not provide.
    fn script(self, app: &App) -> Option<&str> {
        match self {
            Action::Install => Some(&app.install_script),
            Action::Uninstall => app.uninstall_script.as_deref(),
        }
    }

    /// Verb used in buttons and dialogs.
    fn verb(self) -> &'static str {
        match self {
            Action::Install => "Install",
            Action::Uninstall => "Uninstall",
        }
    }
}

/// Main window state. Lives for the whole program inside an `Rc`; all
/// mutation happens on the GTK main thread.
struct Ui {
    window: ApplicationWindow,
    categories: Vec<Category>,
    statuses: RefCell<HashMap<PathBuf, Status>>,
    selected_cat: Cell<Option<usize>>,
    /// Category to go back to when a search is cleared.
    last_cat: Cell<usize>,
    /// Apps shown in the app list, in row order.
    visible: RefCell<Vec<AppPos>>,
    selected_app: Cell<Option<usize>>,
    search: SearchEntry,
    running: Cell<bool>,
    app_status_labels: RefCell<Vec<Label>>,
    cat_list: ListBox,
    app_list: ListBox,
    stack: Stack,
    placeholder: Label,
    detail: ScrolledWindow,
    terminal: vte4::Terminal,
    last_outcome: RefCell<Option<String>>,
    status_label: Label,
    install_btn: Button,
    uninstall_btn: Button,
    view_btn: Button,
    github_btn: Button,
    homepage_btn: Button,
    back_btn: Button,
}

/// Shows a dimmed, centered message (used for placeholders and errors).
fn message_label(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.set_wrap(true);
    label.set_justify(gtk4::Justification::Center);
    label.add_css_class("dim-label");
    label
}

/// Header with the DreamOS logo and the tool title.
fn build_header() -> GtkBox {
    let header = GtkBox::new(Orientation::Horizontal, 12);
    header.set_margin_top(12);
    header.set_margin_bottom(12);
    header.set_margin_start(16);
    header.set_margin_end(16);
    let logo_bytes = glib::Bytes::from_static(LOGO_BYTES);
    let logo_texture =
        gtk4::gdk::Texture::from_bytes(&logo_bytes).expect("assets/ball.png is a valid, embedded PNG");
    let logo = Image::from_paintable(Some(&logo_texture));
    logo.set_pixel_size(48);
    let title = Label::new(None);
    title.set_markup("<span size='x-large' weight='bold'>DreamOS Install</span>");
    header.append(&logo);
    header.append(&title);
    header
}

/// Wraps a widget in a vertically scrolling container of fixed width.
fn side_column(child: &impl IsA<gtk4::Widget>, width: i32) -> ScrolledWindow {
    let scrolled = ScrolledWindow::new();
    scrolled.set_policy(PolicyType::Never, PolicyType::Automatic);
    scrolled.set_size_request(width, -1);
    scrolled.set_child(Some(child));
    scrolled
}

/// True when the process already runs as root, so no elevation is needed.
fn _is_root() -> bool {
    // SAFETY: geteuid() is a pure syscall with no preconditions.
    unsafe { libc::geteuid() == 0 }
}

/// Decodes a waitpid() status from VTE's child-exited signal into a
/// human readable outcome and a success flag.
fn describe_exit(wait_status: i32) -> (String, bool) {
    if libc::WIFEXITED(wait_status) {
        let code = libc::WEXITSTATUS(wait_status);
        if code == 0 {
            return ("Completed successfully".to_string(), true);
        }
        return (format!("Failed (exit code {code})"), false);
    }
    if libc::WIFSIGNALED(wait_status) {
        return (format!("Killed by signal {}", libc::WTERMSIG(wait_status)), false);
    }
    (format!("Ended with status {wait_status}"), false)
}

/// Builds one row of the app list: icon, name (plus the category when
/// searching), summary and a status label.
/// Returns the row and its status label (updated as checks complete).
fn app_row(app: &App, category: Option<&str>) -> (ListBoxRow, Label) {
    let row_box = GtkBox::new(Orientation::Horizontal, 8);
    row_box.set_margin_top(6);
    row_box.set_margin_bottom(6);
    row_box.set_margin_start(8);
    row_box.set_margin_end(8);

    if let Some(icon_path) = &app.icon {
        let icon = Image::from_file(icon_path);
        icon.set_pixel_size(APP_ICON_SIZE);
        icon.set_valign(Align::Center);
        row_box.append(&icon);
    }

    let texts = GtkBox::new(Orientation::Vertical, 2);
    texts.set_hexpand(true);
    let name = Label::new(None);
    let tag = category.map(|c| format!("  <span weight='normal' alpha='60%'>{}</span>", escape_markup(c)));
    name.set_markup(&format!("<b>{}</b>{}", escape_markup(&app.name), tag.unwrap_or_default()));
    name.set_xalign(0.0);
    texts.append(&name);
    if !app.summary.is_empty() {
        let summary = Label::new(Some(&app.summary));
        summary.set_xalign(0.0);
        summary.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        summary.add_css_class("dim-label");
        texts.append(&summary);
    }
    row_box.append(&texts);

    let status = Label::new(None);
    status.set_valign(Align::Center);
    status.add_css_class("caption");
    row_box.append(&status);

    let row = ListBoxRow::new();
    row.set_child(Some(&row_box));
    (row, status)
}

/// Shows `source` in a modal, read-only window with shell syntax
/// highlighting, line numbers and a theme matching the desktop style.
fn show_script_window(parent: &ApplicationWindow, title: &str, source: &str) {
    let buffer = sourceview5::Buffer::new(None);
    buffer.set_text(source);
    buffer.set_language(sourceview5::LanguageManager::default().language("sh").as_ref());
    buffer.set_highlight_syntax(true);
    let scheme_id = if gtk4::Settings::default().is_some_and(|s| {
        s.is_gtk_application_prefer_dark_theme()
            || s.gtk_theme_name().is_some_and(|name| name.to_lowercase().contains("dark"))
    }) {
        SCHEME_DARK
    } else {
        SCHEME_LIGHT
    };
    buffer.set_style_scheme(sourceview5::StyleSchemeManager::default().scheme(scheme_id).as_ref());

    let view = sourceview5::View::with_buffer(&buffer);
    view.set_editable(false);
    view.set_cursor_visible(false);
    view.set_monospace(true);
    view.set_show_line_numbers(true);
    view.set_highlight_current_line(false);
    view.set_top_margin(8);
    view.set_bottom_margin(8);

    let scroller = ScrolledWindow::new();
    scroller.set_child(Some(&view));
    scroller.set_hexpand(true);
    scroller.set_vexpand(true);

    let window = gtk4::Window::builder()
        .title(title)
        .modal(true)
        .transient_for(parent)
        .default_width(SCRIPT_WINDOW_WIDTH)
        .default_height(SCRIPT_WINDOW_HEIGHT)
        .child(&scroller)
        .build();

    // Escape closes the viewer.
    let keys = gtk4::EventControllerKey::new();
    let closing = window.clone();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Escape {
            closing.close();
            return Propagation::Stop;
        }
        Propagation::Proceed
    });
    window.add_controller(keys);
    window.present();
}

impl Ui {
    /// Currently selected app, if any (also while a search is shown).
    fn current_app(&self) -> Option<&App> {
        let (cat_idx, app_idx) = *self.visible.borrow().get(self.selected_app.get()?)?;
        self.categories.get(cat_idx)?.apps.get(app_idx)
    }

    /// Cached status of an app (Checking when no check has finished yet).
    fn status_of(&self, app: &App) -> Status {
        self.statuses.borrow().get(&app.dir).cloned().unwrap_or(Status::Checking)
    }

    /// Shows the apps of category `idx` (no-op when it is already
    /// selected).
    fn select_category(self: &Rc<Self>, idx: usize) {
        // GtkListBox re-emits row-selected for the already selected row when
        // focus lands on it (e.g. after the Back button hides itself).
        if self.selected_cat.get() == Some(idx) {
            return;
        }
        let Some(category) = self.categories.get(idx) else {
            return;
        };
        self.selected_cat.set(Some(idx));
        self.last_cat.set(idx);
        let entries = (0..category.apps.len()).map(|app_idx| (idx, app_idx)).collect();
        self.show_apps(entries, false);
    }

    /// Filters the apps of every category by name. A blank query goes back
    /// to the last selected category.
    fn apply_search(self: &Rc<Self>, query: &str) {
        if query.trim().is_empty() {
            // A category is already shown: nothing to restore.
            if self.selected_cat.get().is_some() {
                return;
            }
            if let Some(row) = self.cat_list.row_at_index(self.last_cat.get() as i32) {
                self.cat_list.select_row(Some(&row));
            }
            return;
        }
        self.selected_cat.set(None);
        self.cat_list.unselect_all();
        self.show_apps(catalog::search(&self.categories, query), true);
    }

    /// Fills the app list with `entries`, resets the selection and starts
    /// the checks of the apps that have no cached status yet.
    fn show_apps(self: &Rc<Self>, entries: Vec<AppPos>, show_category: bool) {
        self.selected_app.set(None);
        self.app_status_labels.borrow_mut().clear();
        while let Some(child) = self.app_list.first_child() {
            self.app_list.remove(&child);
        }
        let mut labels = Vec::new();
        let mut unchecked = Vec::new();
        for &(cat_idx, app_idx) in &entries {
            let category = &self.categories[cat_idx];
            let app = &category.apps[app_idx];
            let (row, label) = app_row(app, show_category.then_some(category.name.as_str()));
            self.app_list.append(&row);
            labels.push(label);
            if !self.statuses.borrow().contains_key(&app.dir) {
                unchecked.push((app.dir.clone(), app.check_script.clone()));
            }
        }
        *self.visible.borrow_mut() = entries;
        *self.app_status_labels.borrow_mut() = labels;
        self.start_checks(unchecked);
        let empty = show_category && self.visible.borrow().is_empty();
        self.show_placeholder(if empty {
            "No app matches the search."
        } else {
            "Select an app to see its description."
        });
        self.refresh();
    }

    /// Shows app `idx` of the app list in the detail pane.
    fn select_app(self: &Rc<Self>, idx: usize) {
        self.selected_app.set(Some(idx));
        let Some(app) = self.current_app() else {
            return;
        };
        let readme = app.dir.join(&app.readme);
        match std::fs::read_to_string(&readme) {
            Ok(source) => {
                let content = render_document(&markdown::parse(&source), &app.dir);
                self.detail.set_child(Some(&content));
                self.detail.vadjustment().set_value(0.0);
                self.stack.set_visible_child_name(PAGE_DETAIL);
            }
            Err(e) => self.show_placeholder(&format!("Cannot read {}: {e}", readme.display())),
        }
        self.refresh();
    }

    /// Shows a message instead of the app detail.
    fn show_placeholder(&self, text: &str) {
        self.placeholder.set_text(text);
        self.stack.set_visible_child_name(PAGE_PLACEHOLDER);
    }

    /// Runs the check script of every `(app dir, check script)` pair in
    /// `checks` on background threads and updates the cache and the UI as
    /// results arrive.
    fn start_checks(self: &Rc<Self>, checks: Vec<(PathBuf, String)>) {
        if checks.is_empty() {
            return;
        }
        let (tx, rx) = mpsc::channel::<(PathBuf, Status)>();
        for (dir, check_script) in &checks {
            self.statuses.borrow_mut().insert(dir.clone(), Status::Checking);
            let tx = tx.clone();
            let dir = dir.clone();
            let check_script = check_script.clone();
            std::thread::spawn(move || {
                let status = run_check(&dir, &check_script, CHECK_TIMEOUT);
                // The receiver only goes away with the main loop; nothing to do then.
                let _ = tx.send((dir, status));
            });
        }
        drop(tx);
        let ui = Rc::clone(self);
        let mut pending = checks.len();
        glib::source::timeout_add_local(CHECK_POLL, move || {
            while let Ok((dir, status)) = rx.try_recv() {
                ui.statuses.borrow_mut().insert(dir, status);
                pending -= 1;
            }
            ui.refresh();
            if pending == 0 {
                return ControlFlow::Break;
            }
            ControlFlow::Continue
        });
    }

    /// Syncs list status labels, the action bar and widget sensitivity
    /// with the current state.
    fn refresh(&self) {
        let running = self.running.get();
        self.cat_list.set_sensitive(!running);
        self.app_list.set_sensitive(!running);

        self.search.set_sensitive(!running);

        for (&(cat_idx, app_idx), label) in self.visible.borrow().iter().zip(self.app_status_labels.borrow().iter()) {
            label.set_text(self.status_of(&self.categories[cat_idx].apps[app_idx]).label());
        }

        self.install_btn.set_visible(false);
        self.uninstall_btn.set_visible(false);
        self.view_btn.set_visible(false);
        self.github_btn.set_visible(false);
        self.homepage_btn.set_visible(false);
        if running {
            self.status_label.set_text("Running...");
            return;
        }
        let Some(app) = self.current_app() else {
            self.status_label.set_text("");
            self.status_label.set_tooltip_text(None);
            return;
        };
        let status = self.status_of(app);
        let outcome = match self.last_outcome.borrow().as_deref() {
            Some(outcome) => format!("{outcome}. Now: "),
            None => String::new(),
        };
        self.status_label.set_markup(&format!(
            "<b>{}</b>: {}{}",
            escape_markup(&app.name),
            escape_markup(&outcome),
            escape_markup(status.label())
        ));
        match &status {
            Status::CheckFailed(reason) => self.status_label.set_tooltip_text(Some(reason)),
            _ => self.status_label.set_tooltip_text(None),
        }
        // Showing a finished script's output: only Back is offered.
        if self.back_btn.is_visible() {
            return;
        }
        self.install_btn.set_visible(matches!(status, Status::NotInstalled | Status::CheckFailed(_)));
        self.uninstall_btn.set_visible(status == Status::Installed && app.uninstall_script.is_some());
        self.view_btn.set_visible(self.install_btn.is_visible() || self.uninstall_btn.is_visible());
        self.github_btn.set_visible(app.github.is_some());
        self.homepage_btn.set_visible(app.homepage.is_some());
    }

    /// Script the visible action button would run: install when offered,
    /// otherwise uninstall.
    fn offered_action(&self) -> Action {
        if self.install_btn.is_visible() {
            Action::Install
        } else {
            Action::Uninstall
        }
    }

    /// Opens a modal read-only window with the highlighted source of the
    /// script that the offered action is about to run.
    fn view_script(&self) {
        let Some(app) = self.current_app() else {
            return;
        };
        let Some(script) = self.offered_action().script(app) else {
            return;
        };
        let path = app.dir.join(script);
        match std::fs::read_to_string(&path) {
            Ok(source) => show_script_window(&self.window, &format!("{} - {script}", app.name), &source),
            Err(e) => {
                let dialog = AlertDialog::builder()
                    .modal(true)
                    .message(format!("Cannot read {script}"))
                    .detail(format!("{}: {e}", path.display()))
                    .build();
                dialog.show(Some(&self.window));
            }
        }
    }

    /// Opens an http(s) link of the selected app in the default browser;
    /// `pick` chooses which link.
    fn open_link(&self, pick: fn(&App) -> Option<&String>) {
        let Some(url) = self.current_app().and_then(pick) else {
            return;
        };
        let parent = self.window.clone();
        gtk4::UriLauncher::new(url).launch(Some(&self.window), None::<&gio::Cancellable>, {
            let url = url.clone();
            move |result| {
                let Err(e) = result else {
                    return;
                };
                AlertDialog::builder()
                    .modal(true)
                    .message("Cannot open the link")
                    .detail(format!("{url}: {e}"))
                    .build()
                    .show(Some(&parent));
            }
        });
    }

    /// Asks for confirmation, then runs the action's script for the
    /// selected app.
    fn confirm_and_run(self: &Rc<Self>, action: Action) {
        let Some(app) = self.current_app() else {
            return;
        };
        let Some(script) = action.script(app) else {
            return;
        };
        let dialog = AlertDialog::builder()
            .modal(true)
            .message(format!("{} {}?", action.verb(), app.name))
            .detail(format!(
                "This runs {} from {} as root. The administrator password is asked next.",
                script,
                app.dir.display()
            ))
            .buttons(["Cancel", action.verb()])
            .cancel_button(0)
            .default_button(1)
            .build();
        let ui = Rc::clone(self);
        dialog.choose(Some(&self.window), None::<&gio::Cancellable>, move |choice| {
            if choice != Ok(1) {
                return;
            }
            ui.run_script(action);
        });
    }

    /// Runs the action's script for the selected app in the embedded
    /// terminal. Completion is handled by `finish_script`.
    fn run_script(self: &Rc<Self>, action: Action) {
        let Some(app) = self.current_app().cloned() else {
            return;
        };
        let Some(script) = action.script(&app) else {
            return;
        };
        self.running.set(true);
        self.terminal.reset(true, true);
        self.terminal.feed(
            format!("\x1b[1m==> {} {}: bash {}\x1b[0m\r\n\r\n", action.verb(), app.name, script)
                .as_bytes(),
        );
        self.last_outcome.replace(None);
        self.stack.set_visible_child_name(PAGE_TERMINAL);
        self.refresh();

        let dir = app.dir.to_string_lossy().into_owned();
        let user = invoking_user().unwrap_or_default();
        let elevate = std::env::var_os(SKIP_ROOT_ENV).is_none() && !_is_root();
        let argv = script_argv(&app.dir, script, &[(APP_DIR_ENV, &dir), (USER_ENV, &user)], elevate);
        let argv: Vec<String> = argv.iter().map(|a| a.to_string_lossy().into_owned()).collect();
        let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
        // An explicit copy of our environment: an empty envv could be read as "no variables".
        let environ: Vec<String> = std::env::vars().map(|(k, v)| format!("{k}={v}")).collect();
        let environ: Vec<&str> = environ.iter().map(String::as_str).collect();
        let ui = Rc::clone(self);
        self.terminal.spawn_async(
            vte4::PtyFlags::DEFAULT,
            Some(&dir),
            &argv,
            &environ,
            glib::SpawnFlags::SEARCH_PATH,
            || {},
            -1,
            None::<&gio::Cancellable>,
            move |result| {
                if let Err(e) = result {
                    ui.finish_script(&format!("Failed to start: {e}"));
                }
            },
        );
    }

    /// Ends a script run: shows the outcome, re-enables navigation and
    /// re-checks the app.
    fn finish_script(self: &Rc<Self>, outcome: &str) {
        self.running.set(false);
        self.terminal.feed(format!("\r\n\x1b[1m==> {outcome}\x1b[0m\r\n").as_bytes());
        self.last_outcome.replace(Some(outcome.to_string()));
        self.back_btn.set_visible(true);
        if let Some(app) = self.current_app() {
            self.statuses.borrow_mut().remove(&app.dir);
            self.start_checks(vec![(app.dir.clone(), app.check_script.clone())]);
        }
        self.refresh();
    }

    /// Leaves the terminal and returns to the app detail.
    fn back_to_detail(self: &Rc<Self>) {
        self.back_btn.set_visible(false);
        self.last_outcome.replace(None);
        match self.selected_app.get() {
            Some(idx) => self.select_app(idx),
            None => self.refresh(),
        }
    }
}

/// Builds the main window for the scanned catalog (or a scan error).
pub fn build_window(app: &Application, root: &Path, scan: Result<ScanResult, String>) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("DreamOS Install")
        .default_width(1100)
        .default_height(720)
        .build();

    let outer = GtkBox::new(Orientation::Vertical, 0);
    let search = SearchEntry::new();
    search.set_placeholder_text(Some("Search apps (Ctrl+F)"));
    search.set_width_chars(SEARCH_WIDTH_CHARS);
    search.set_hexpand(true);
    search.set_halign(Align::End);
    search.set_valign(Align::Center);
    let header = build_header();
    header.append(&search);
    outer.append(&header);
    outer.append(&Separator::new(Orientation::Horizontal));
    window.set_child(Some(&outer));

    let categories = match scan {
        Ok(result) => result.categories,
        Err(e) => {
            let error = message_label(&format!("Cannot load the app catalog.\n\n{e}"));
            error.set_vexpand(true);
            outer.append(&error);
            return window;
        }
    };
    if categories.is_empty() {
        let empty = message_label(&format!("No installable apps found in {}.", root.display()));
        empty.set_vexpand(true);
        outer.append(&empty);
        return window;
    }

    let cat_list = ListBox::new();
    cat_list.set_selection_mode(SelectionMode::Single);
    cat_list.add_css_class("navigation-sidebar");
    for category in &categories {
        let label = Label::new(Some(&category.name));
        label.set_xalign(0.0);
        label.set_margin_top(6);
        label.set_margin_bottom(6);
        label.set_margin_start(8);
        cat_list.append(&label);
    }
    let app_list = ListBox::new();
    app_list.set_selection_mode(SelectionMode::Single);

    let placeholder = message_label("");
    let detail = ScrolledWindow::new();
    detail.set_policy(PolicyType::Never, PolicyType::Automatic);

    let terminal = vte4::Terminal::new();
    terminal.set_scrollback_lines(SCROLLBACK_LINES);
    terminal.set_vexpand(true);
    terminal.set_hexpand(true);
    let terminal_page = ScrolledWindow::new();
    terminal_page.set_child(Some(&terminal));

    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.add_named(&placeholder, Some(PAGE_PLACEHOLDER));
    stack.add_named(&detail, Some(PAGE_DETAIL));
    stack.add_named(&terminal_page, Some(PAGE_TERMINAL));

    let panes = GtkBox::new(Orientation::Horizontal, 0);
    panes.set_vexpand(true);
    panes.append(&side_column(&cat_list, 170));
    panes.append(&Separator::new(Orientation::Vertical));
    panes.append(&side_column(&app_list, 320));
    panes.append(&Separator::new(Orientation::Vertical));
    panes.append(&stack);
    outer.append(&panes);
    outer.append(&Separator::new(Orientation::Horizontal));

    let status_label = Label::new(None);
    status_label.set_xalign(0.0);
    status_label.set_hexpand(true);
    let back_btn = Button::with_label("Back");
    back_btn.set_visible(false);
    let view_btn = Button::with_label("View script");
    view_btn.set_visible(false);
    let github_btn = Button::with_label("Github Page");
    github_btn.set_visible(false);
    let homepage_btn = Button::with_label("Home page");
    homepage_btn.set_visible(false);
    let uninstall_btn = Button::with_label(Action::Uninstall.verb());
    uninstall_btn.add_css_class("destructive-action");
    let install_btn = Button::with_label(Action::Install.verb());
    install_btn.add_css_class("suggested-action");
    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.set_margin_top(10);
    actions.set_margin_bottom(10);
    actions.set_margin_start(16);
    actions.set_margin_end(16);
    // Keep the bar height stable when every button is hidden.
    actions.set_height_request(34);
    actions.append(&status_label);
    actions.append(&back_btn);
    actions.append(&view_btn);
    actions.append(&github_btn);
    actions.append(&homepage_btn);
    actions.append(&uninstall_btn);
    actions.append(&install_btn);
    outer.append(&actions);

    let ui = Rc::new(Ui {
        window: window.clone(),
        categories,
        statuses: RefCell::new(HashMap::new()),
        selected_cat: Cell::new(None),
        last_cat: Cell::new(0),
        visible: RefCell::new(Vec::new()),
        selected_app: Cell::new(None),
        search,
        running: Cell::new(false),
        app_status_labels: RefCell::new(Vec::new()),
        cat_list,
        app_list,
        stack,
        placeholder,
        detail,
        terminal,
        last_outcome: RefCell::new(None),
        status_label,
        install_btn,
        uninstall_btn,
        view_btn,
        github_btn,
        homepage_btn,
        back_btn,
    });
    connect_signals(&ui);

    if let Some(first) = ui.cat_list.row_at_index(0) {
        ui.cat_list.select_row(Some(&first));
    }
    window
}

/// Wires list selection, buttons, terminal exit and window close.
fn connect_signals(ui: &Rc<Ui>) {
    let weak = Rc::downgrade(ui);
    ui.cat_list.connect_row_selected(move |_, row| {
        let (Some(ui), Some(row)) = (weak.upgrade(), row) else {
            return;
        };
        ui.select_category(row.index() as usize);
        // Picking a category leaves the search mode.
        ui.search.set_text("");
    });

    let weak = Rc::downgrade(ui);
    ui.search.connect_search_changed(move |entry| {
        if let Some(ui) = weak.upgrade() {
            ui.apply_search(&entry.text());
        }
    });

    let weak = Rc::downgrade(ui);
    ui.search.connect_stop_search(move |entry| {
        if weak.upgrade().is_some() {
            entry.set_text("");
        }
    });

    // Ctrl+F focuses the search box.
    let keys = gtk4::EventControllerKey::new();
    let weak = Rc::downgrade(ui);
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        let is_find = key == gtk4::gdk::Key::f && modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK);
        match weak.upgrade() {
            Some(ui) if is_find && !ui.running.get() => {
                ui.search.grab_focus();
                Propagation::Stop
            }
            _ => Propagation::Proceed,
        }
    });
    ui.window.add_controller(keys);

    let weak = Rc::downgrade(ui);
    ui.app_list.connect_row_selected(move |_, row| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        match row {
            Some(row) => ui.select_app(row.index() as usize),
            None => {
                ui.selected_app.set(None);
                ui.refresh();
            }
        }
    });

    let weak = Rc::downgrade(ui);
    ui.install_btn.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.confirm_and_run(Action::Install);
        }
    });

    let weak = Rc::downgrade(ui);
    ui.uninstall_btn.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.confirm_and_run(Action::Uninstall);
        }
    });

    let weak = Rc::downgrade(ui);
    ui.view_btn.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.view_script();
        }
    });

    let weak = Rc::downgrade(ui);
    ui.github_btn.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.open_link(|app| app.github.as_ref());
        }
    });

    let weak = Rc::downgrade(ui);
    ui.homepage_btn.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.open_link(|app| app.homepage.as_ref());
        }
    });

    let weak = Rc::downgrade(ui);
    ui.back_btn.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.back_to_detail();
        }
    });

    let weak = Rc::downgrade(ui);
    ui.terminal.connect_child_exited(move |_, wait_status| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        if !ui.running.get() {
            return;
        }
        let (outcome, _) = describe_exit(wait_status);
        ui.finish_script(&outcome);
    });

    // The window owns the Ui through this handler, keeping it alive for the
    // whole program; the other handlers only hold weak references.
    let owner = Rc::clone(ui);
    ui.window.connect_close_request(move |window| {
        if !owner.running.get() {
            return Propagation::Proceed;
        }
        let dialog = AlertDialog::builder()
            .modal(true)
            .message("A script is still running")
            .detail("Closing now interrupts it and may leave the app half installed.")
            .buttons(["Keep running", "Close anyway"])
            .cancel_button(0)
            .default_button(0)
            .build();
        let ui = Rc::clone(&owner);
        let to_close = window.clone();
        dialog.choose(Some(window), None::<&gio::Cancellable>, move |choice| {
            if choice != Ok(1) {
                return;
            }
            ui.running.set(false);
            to_close.close();
        });
        Propagation::Stop
    });
}

/// Scans the catalog and logs skipped apps.
pub fn load_catalog(root: &Path) -> Result<ScanResult, String> {
    let result = catalog::scan(root).map_err(|e| e.to_string())?;
    for warning in &result.warnings {
        eprintln!("dreamos-install: skipped {warning}");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_exit_decodes_wait_status() {
        assert_eq!(describe_exit(0), ("Completed successfully".to_string(), true));
        assert_eq!(describe_exit(2 << 8), ("Failed (exit code 2)".to_string(), false));
        assert_eq!(describe_exit(9), ("Killed by signal 9".to_string(), false));
    }
}
