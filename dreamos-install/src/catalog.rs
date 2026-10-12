use crate::markdown;
use crate::CatalogError;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub const DEFAULT_ROOT: &str = "/usr/share/dreamos-install/apps";
pub const ROOT_ENV: &str = "DREAMOS_INSTALL_DIR";
pub const README_FILE: &str = "README.md";
pub const INSTALL_SCRIPT: &str = "install.sh";
pub const CHECK_SCRIPT: &str = "check.sh";
pub const UNINSTALL_SCRIPT: &str = "uninstall.sh";

/// One installable app: a folder with README.md, install.sh, check.sh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub dir: PathBuf,
    pub name: String,
    pub summary: String,
    pub screenshot: Option<PathBuf>,
    pub has_uninstall: bool,
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

/// Loads one app folder; returns the reason when the folder is not a
/// valid app.
pub fn load_app(dir: &Path) -> Result<App, String> {
    for required in [README_FILE, INSTALL_SCRIPT, CHECK_SCRIPT] {
        if !dir.join(required).is_file() {
            return Err(format!("{}: missing {required}", dir.display()));
        }
    }
    let source = std::fs::read_to_string(dir.join(README_FILE))
        .map_err(|e| format!("{}: cannot read {README_FILE}: {e}", dir.display()))?;
    let doc = markdown::parse(&source);
    Ok(App {
        dir: dir.to_path_buf(),
        name: doc.title.unwrap_or_else(|| _folder_name(dir)),
        summary: doc.summary.unwrap_or_default(),
        screenshot: doc.first_local_image.map(|src| dir.join(src)),
        has_uninstall: dir.join(UNINSTALL_SCRIPT).is_file(),
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

    fn make_app(dir: &Path, readme: &str, uninstall: bool) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(README_FILE), readme).unwrap();
        fs::write(dir.join(INSTALL_SCRIPT), "true\n").unwrap();
        fs::write(dir.join(CHECK_SCRIPT), "true\n").unwrap();
        if uninstall {
            fs::write(dir.join(UNINSTALL_SCRIPT), "true\n").unwrap();
        }
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
        make_app(&root.join("graphics/krita"), "# Krita\n\nPainting.\n", false);
        make_app(&root.join("graphics/gimp"), "# GIMP\n\nRaster editor.\n\n![s](shot.png)\n", true);
        make_app(&root.join("development/zed"), "no title here\n", false);

        let result = scan(&root).unwrap();
        assert!(result.warnings.is_empty());
        let names: Vec<_> = result.categories.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Development", "Graphics"]);

        let dev = &result.categories[0].apps[0];
        assert_eq!(dev.name, "zed");
        assert_eq!(dev.summary, "no title here");

        let graphics = &result.categories[1].apps;
        assert_eq!(graphics[0].name, "GIMP");
        assert_eq!(graphics[0].summary, "Raster editor.");
        assert_eq!(graphics[0].screenshot, Some(root.join("graphics/gimp/shot.png")));
        assert!(graphics[0].has_uninstall);
        assert_eq!(graphics[1].name, "Krita");
        assert!(!graphics[1].has_uninstall);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn skips_invalid_apps_hidden_dirs_and_empty_categories() {
        let root = temp_root();
        make_app(&root.join("system/ok"), "# Ok\n", false);
        fs::create_dir_all(root.join("system/broken")).unwrap();
        fs::write(root.join("system/broken").join(README_FILE), "# Broken\n").unwrap();
        make_app(&root.join("system/.hidden"), "# Hidden\n", false);
        make_app(&root.join(".git/x"), "# X\n", false);
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
}
