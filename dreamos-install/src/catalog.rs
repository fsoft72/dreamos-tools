use crate::markdown;
use crate::CatalogError;
use serde::Deserialize;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

pub const DEFAULT_ROOT: &str = "/usr/share/dreamos-install/apps";
pub const ROOT_ENV: &str = "DREAMOS_INSTALL_DIR";
pub const MANIFEST_FILE: &str = "manifest.json";
/// Default values of the optional manifest keys (paths relative to the
/// manifest).
pub const DEFAULT_ICON: &str = "icon.png";
pub const DEFAULT_README: &str = "README.md";
pub const DEFAULT_CHECK: &str = "check.sh";
pub const DEFAULT_INSTALL: &str = "install.sh";

/// Raw content of an app's `manifest.json`. Only `name` is mandatory; the
/// path keys default to the conventional file names.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    name: String,
    icon: Option<String>,
    readme: Option<String>,
    check: Option<String>,
    install: Option<String>,
    uninstall: Option<String>,
    screenshot: Option<String>,
    github: Option<String>,
    homepage: Option<String>,
}

/// One installable app: a folder with a `manifest.json` that names the
/// README, the check and install scripts and the optional icon,
/// screenshot, uninstall script and project links. Every path is stored
/// relative to `dir`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub dir: PathBuf,
    pub name: String,
    pub summary: String,
    pub readme: String,
    pub check_script: String,
    pub install_script: String,
    pub uninstall_script: Option<String>,
    pub screenshot: Option<PathBuf>,
    pub icon: Option<PathBuf>,
    pub github: Option<String>,
    pub homepage: Option<String>,
}

/// A category folder and the valid apps found inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub dir: PathBuf,
    pub name: String,
    pub apps: Vec<App>,
}

/// Result of a catalog scan: the valid categories plus a human readable
/// warning for every app folder that was skipped.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanResult {
    pub categories: Vec<Category>,
    pub warnings: Vec<String>,
}

/// Picks the catalog root: CLI argument first, then the env var value,
/// then the built-in default.
pub fn resolve_root(arg: Option<OsString>, env: Option<OsString>) -> PathBuf {
    arg.or(env).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(DEFAULT_ROOT))
}

/// Folder name with the first letter capitalized ("graphics" -> "Graphics").
pub fn display_name(folder: &str) -> String {
    let mut chars = folder.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first.to_uppercase().chain(chars).collect()
}

/// Lists the visible (non-hidden) sub-directories of `dir`, sorted by name.
fn _sub_dirs(dir: &Path) -> Result<Vec<PathBuf>, CatalogError> {
    let entries = std::fs::read_dir(dir).map_err(|e| CatalogError::ReadDir {
        path: dir.display().to_string(),
        detail: e.to_string(),
    })?;
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_dir())
        .filter(|path| !_folder_name(path).starts_with('.'))
        .collect();
    dirs.sort();
    Ok(dirs)
}

/// Last path component as a String (lossy).
fn _folder_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Rejects manifest paths that are absolute or climb out of the app folder.
fn _check_rel_path(key: &str, value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.is_empty() || path.components().any(|c| !matches!(c, Component::Normal(_) | Component::CurDir)) {
        return Err(format!("'{key}' must be a path inside the app folder, got '{value}'"));
    }
    Ok(())
}

/// Rejects links that are not http(s) URLs.
fn _check_url(key: &str, value: &str) -> Result<(), String> {
    if value.starts_with("https://") || value.starts_with("http://") {
        return Ok(());
    }
    Err(format!("'{key}' must be an http(s) URL, got '{value}'"))
}

/// Parses and validates the manifest of the app in `dir`.
fn _read_manifest(dir: &Path) -> Result<Manifest, String> {
    let path = dir.join(MANIFEST_FILE);
    let source = std::fs::read_to_string(&path).map_err(|e| format!("{}: cannot read {MANIFEST_FILE}: {e}", dir.display()))?;
    let manifest: Manifest =
        serde_json::from_str(&source).map_err(|e| format!("{}: invalid {MANIFEST_FILE}: {e}", dir.display()))?;
    let wrap = |e: String| format!("{}: {MANIFEST_FILE}: {e}", dir.display());
    if manifest.name.trim().is_empty() {
        return Err(wrap("'name' must not be empty".into()));
    }
    let paths = [
        ("icon", &manifest.icon),
        ("readme", &manifest.readme),
        ("check", &manifest.check),
        ("install", &manifest.install),
        ("uninstall", &manifest.uninstall),
        ("screenshot", &manifest.screenshot),
    ];
    for (key, value) in paths {
        if let Some(value) = value {
            _check_rel_path(key, value).map_err(wrap)?;
        }
    }
    for (key, value) in [("github", &manifest.github), ("homepage", &manifest.homepage)] {
        if let Some(value) = value {
            _check_url(key, value).map_err(wrap)?;
        }
    }
    Ok(manifest)
}

/// Loads one app folder; returns the reason when the folder is not a
/// valid app.
pub fn load_app(dir: &Path) -> Result<App, String> {
    let manifest = _read_manifest(dir)?;
    let readme = manifest.readme.unwrap_or_else(|| DEFAULT_README.into());
    let check_script = manifest.check.unwrap_or_else(|| DEFAULT_CHECK.into());
    let install_script = manifest.install.unwrap_or_else(|| DEFAULT_INSTALL.into());
    for required in [&readme, &check_script, &install_script] {
        if !dir.join(required).is_file() {
            return Err(format!("{}: missing {required}", dir.display()));
        }
    }
    let source = std::fs::read_to_string(dir.join(&readme))
        .map_err(|e| format!("{}: cannot read {readme}: {e}", dir.display()))?;
    let doc = markdown::parse(&source);
    let icon = dir.join(manifest.icon.as_deref().unwrap_or(DEFAULT_ICON));
    Ok(App {
        dir: dir.to_path_buf(),
        name: manifest.name,
        summary: doc.summary.unwrap_or_default(),
        readme,
        check_script,
        install_script,
        uninstall_script: manifest.uninstall.filter(|script| dir.join(script).is_file()),
        screenshot: manifest.screenshot.map(|shot| dir.join(shot)),
        icon: Some(icon).filter(|path| path.is_file()),
        github: manifest.github,
        homepage: manifest.homepage,
    })
}

/// Scans `root` for category folders and their apps. Fails only when the
/// root itself cannot be read; invalid apps become warnings.
pub fn scan(root: &Path) -> Result<ScanResult, CatalogError> {
    let mut result = ScanResult::default();
    for cat_dir in _sub_dirs(root)? {
        let app_dirs = match _sub_dirs(&cat_dir) {
            Ok(dirs) => dirs,
            Err(e) => {
                result.warnings.push(e.to_string());
                continue;
            }
        };
        let mut apps = Vec::new();
        for app_dir in app_dirs {
            match load_app(&app_dir) {
                Ok(app) => apps.push(app),
                Err(reason) => result.warnings.push(reason),
            }
        }
        if apps.is_empty() {
            continue;
        }
        apps.sort_by_key(|app| app.name.to_lowercase());
        result.categories.push(Category {
            name: display_name(&_folder_name(&cat_dir)),
            dir: cat_dir,
            apps,
        });
    }
    Ok(result)
}

/// Position of an app in a catalog: (category index, app index).
pub type AppPos = (usize, usize);

/// Apps whose name contains `query`, ignoring case and surrounding
/// whitespace, in catalog order. A blank query matches every app.
pub fn search(categories: &[Category], query: &str) -> Vec<AppPos> {
    let needle = query.trim().to_lowercase();
    let mut found = Vec::new();
    for (cat_idx, category) in categories.iter().enumerate() {
        for (app_idx, app) in category.apps.iter().enumerate() {
            if app.name.to_lowercase().contains(&needle) {
                found.push((cat_idx, app_idx));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Creates a fresh, unique temp directory for one test.
    fn temp_root() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("dreamos-install-cat-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Writes a minimal valid app: `manifest` is the manifest.json body.
    fn make_app_with(dir: &Path, manifest: &str, readme: &str, uninstall: bool) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(MANIFEST_FILE), manifest).unwrap();
        fs::write(dir.join(DEFAULT_README), readme).unwrap();
        fs::write(dir.join(DEFAULT_INSTALL), "true\n").unwrap();
        fs::write(dir.join(DEFAULT_CHECK), "true\n").unwrap();
        if uninstall {
            fs::write(dir.join("uninstall.sh"), "true\n").unwrap();
        }
    }

    /// Like `make_app_with`, with a manifest made of `name` only (plus the
    /// uninstall key when requested).
    fn make_app(dir: &Path, name: &str, readme: &str, uninstall: bool) {
        let extra = if uninstall { r#", "uninstall": "uninstall.sh""# } else { "" };
        make_app_with(dir, &format!(r#"{{"name": "{name}"{extra}}}"#), readme, uninstall);
    }

    #[test]
    fn resolve_root_priority() {
        let arg = Some(OsString::from("/a"));
        let env = Some(OsString::from("/b"));
        assert_eq!(resolve_root(arg, env.clone()), PathBuf::from("/a"));
        assert_eq!(resolve_root(None, env), PathBuf::from("/b"));
        assert_eq!(resolve_root(None, None), PathBuf::from(DEFAULT_ROOT));
    }

    #[test]
    fn display_name_capitalizes() {
        assert_eq!(display_name("graphics"), "Graphics");
        assert_eq!(display_name(""), "");
    }

    #[test]
    fn scans_categories_and_apps() {
        let root = temp_root();
        make_app(&root.join("graphics/krita"), "Krita", "# Krita\n\nPainting.\n", false);
        make_app_with(
            &root.join("graphics/gimp"),
            r#"{"name": "GIMP", "uninstall": "uninstall.sh", "screenshot": "shot.png"}"#,
            "# Other title\n\nRaster editor.\n",
            true,
        );
        make_app(&root.join("development/zed"), "Zed", "no title here\n", false);

        let result = scan(&root).unwrap();
        assert!(result.warnings.is_empty());
        let names: Vec<_> = result.categories.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Development", "Graphics"]);

        let dev = &result.categories[0].apps[0];
        assert_eq!(dev.name, "Zed");
        assert_eq!(dev.summary, "no title here");

        let graphics = &result.categories[1].apps;
        assert_eq!(graphics[0].name, "GIMP");
        assert_eq!(graphics[0].summary, "Raster editor.");
        assert_eq!(graphics[0].screenshot, Some(root.join("graphics/gimp/shot.png")));
        assert_eq!(graphics[0].uninstall_script.as_deref(), Some("uninstall.sh"));
        assert_eq!(graphics[0].icon, None);
        assert_eq!(graphics[1].name, "Krita");
        assert_eq!(graphics[1].uninstall_script, None);
        assert_eq!(graphics[1].screenshot, None);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn manifest_defaults_and_custom_paths() {
        let root = temp_root();
        make_app(&root.join("a/default"), "Default", "# D\n", false);
        let app = load_app(&root.join("a/default")).unwrap();
        assert_eq!((app.readme.as_str(), app.check_script.as_str(), app.install_script.as_str()), ("README.md", "check.sh", "install.sh"));
        assert_eq!((app.github, app.homepage), (None, None));

        let dir = root.join("a/custom");
        make_app_with(
            &dir,
            r#"{"name": "C", "readme": "docs/about.md", "check": "bin/c.sh", "install": "bin/i.sh",
                "github": "https://github.com/x/y", "homepage": "https://x.example"}"#,
            "",
            false,
        );
        fs::create_dir_all(dir.join("docs")).unwrap();
        fs::create_dir_all(dir.join("bin")).unwrap();
        fs::write(dir.join("docs/about.md"), "# C\n\nCustom.\n").unwrap();
        fs::write(dir.join("bin/c.sh"), "true\n").unwrap();
        fs::write(dir.join("bin/i.sh"), "true\n").unwrap();
        let app = load_app(&dir).unwrap();
        assert_eq!((app.readme.as_str(), app.check_script.as_str(), app.install_script.as_str()), ("docs/about.md", "bin/c.sh", "bin/i.sh"));
        assert_eq!(app.summary, "Custom.");
        assert_eq!(app.github.as_deref(), Some("https://github.com/x/y"));
        assert_eq!(app.homepage.as_deref(), Some("https://x.example"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn rejects_bad_manifests() {
        let root = temp_root();
        for (case, manifest, expected) in [
            ("no-manifest", None, "cannot read manifest.json"),
            ("bad-json", Some("{"), "invalid manifest.json"),
            ("no-name", Some("{}"), "invalid manifest.json"),
            ("empty-name", Some(r#"{"name": " "}"#), "'name' must not be empty"),
            ("unknown-key", Some(r#"{"name": "X", "nmae": "y"}"#), "invalid manifest.json"),
            ("escaping", Some(r#"{"name": "X", "install": "../x.sh"}"#), "'install' must be a path inside"),
            ("absolute", Some(r#"{"name": "X", "icon": "/etc/passwd"}"#), "'icon' must be a path inside"),
            ("bad-url", Some(r#"{"name": "X", "github": "javascript:alert(1)"}"#), "'github' must be an http(s) URL"),
            ("no-script", Some(r#"{"name": "X", "check": "nope.sh"}"#), "missing nope.sh"),
        ] {
            let dir = root.join("cat").join(case);
            make_app_with(&dir, "{}", "# X\n", false);
            match manifest {
                Some(body) => fs::write(dir.join(MANIFEST_FILE), body).unwrap(),
                None => fs::remove_file(dir.join(MANIFEST_FILE)).unwrap(),
            }
            let err = load_app(&dir).unwrap_err();
            assert!(err.contains(expected), "{case}: {err}");
        }
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn detects_optional_icon() {
        let root = temp_root();
        make_app(&root.join("graphics/with-icon"), "With", "# With\n", false);
        fs::write(root.join("graphics/with-icon").join(DEFAULT_ICON), "png").unwrap();
        make_app(&root.join("graphics/no-icon"), "No", "# No\n", false);

        let apps = scan(&root).unwrap().categories.remove(0).apps;
        assert_eq!(apps[0].icon, None);
        assert_eq!(apps[1].icon, Some(root.join("graphics/with-icon").join(DEFAULT_ICON)));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn skips_invalid_apps_hidden_dirs_and_empty_categories() {
        let root = temp_root();
        make_app(&root.join("system/ok"), "Ok", "# Ok\n", false);
        make_app(&root.join("system/broken"), "Broken", "# Broken\n", false);
        fs::remove_file(root.join("system/broken").join(DEFAULT_INSTALL)).unwrap();
        make_app(&root.join("system/.hidden"), "Hidden", "# Hidden\n", false);
        make_app(&root.join(".git/x"), "X", "# X\n", false);
        fs::create_dir_all(root.join("empty")).unwrap();
        fs::write(root.join("not-a-dir.txt"), "").unwrap();

        let result = scan(&root).unwrap();
        assert_eq!(result.categories.len(), 1);
        assert_eq!(result.categories[0].apps.len(), 1);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("missing install.sh"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn missing_root_is_an_error() {
        let err = scan(Path::new("/nonexistent/dreamos-install-root")).unwrap_err();
        assert!(err.to_string().contains("/nonexistent/dreamos-install-root"));
    }

    #[test]
    fn search_matches_part_of_name_ignoring_case() {
        let root = temp_root();
        make_app(&root.join("graphics/krita"), "Krita", "# Krita\n", false);
        make_app(&root.join("graphics/inkscape"), "Inkscape", "# Inkscape\n", false);
        make_app(&root.join("video/obs"), "OBS Studio", "# OBS Studio\n", false);
        let categories = scan(&root).unwrap().categories;

        assert_eq!(search(&categories, "SCAPE"), [(0, 0)]);
        assert_eq!(search(&categories, " studio "), [(1, 0)]);
        assert_eq!(search(&categories, "k"), [(0, 0), (0, 1)]);
        assert!(search(&categories, "nothing").is_empty());
        assert_eq!(search(&categories, "  ").len(), 3);
        fs::remove_dir_all(&root).unwrap();
    }
}
