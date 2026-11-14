#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use relm4::RelmApp;
use simpad_control_panel_rs::ui::AppModel;

#[cfg(any(target_os = "windows", test))]
mod portable_env {
    use std::path::{Path, PathBuf};

    /// Locate a portable bundle root directory relative to the executable folder
    /// (supports both single-folder layout and `bin/<exe>` layout).
    pub fn portable_bundle_root(exe_dir: &Path) -> Option<PathBuf> {
        let has_schemas = |dir: &Path| {
            dir.join("share")
                .join("glib-2.0")
                .join("schemas")
                .join("gschemas.compiled")
                .is_file()
        };
        if has_schemas(exe_dir) {
            return Some(exe_dir.to_path_buf());
        }
        exe_dir
            .parent()
            .filter(|parent| has_schemas(parent))
            .map(Path::to_path_buf)
    }

    /// Resolve `@LOADERS_DIR@` placeholders in a `loaders.cache.in` template with
    /// GKeyFile-escaped path separators (`\\` on Windows).
    pub fn render_loaders_cache(template: &str, loaders_dir: &str) -> String {
        let trimmed = loaders_dir.trim_end_matches(['\\', '/']);
        let escaped = trimmed.replace('\\', "\\\\");
        template.replace("@LOADERS_DIR@", &escaped)
    }

    #[cfg(target_os = "windows")]
    fn set_process_env(key: &str, value: impl AsRef<std::ffi::OsStr>) {
        // SAFETY: Invoked once at startup in `main()` before GTK/GLib spawns worker threads.
        let _ = unsafe { relm4::gtk::glib::setenv(key, value, true) };
    }

    #[cfg(target_os = "windows")]
    fn find_pixbuf_abi_dir(pixbuf_root: &Path) -> Option<PathBuf> {
        let entries = std::fs::read_dir(pixbuf_root).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("loaders.cache.in").is_file() {
                return Some(path);
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    pub fn configure_windows() {
        let Ok(exe_path) = std::env::current_exe() else {
            return;
        };
        let Some(exe_dir) = exe_path.parent() else {
            return;
        };
        let Some(root) = portable_bundle_root(exe_dir) else {
            return;
        };

        let share_dir = root.join("share");
        set_process_env(
            "GSETTINGS_SCHEMA_DIR",
            share_dir.join("glib-2.0").join("schemas"),
        );

        let mut data_dirs = std::ffi::OsString::from(share_dir.as_os_str());
        if let Some(existing) = std::env::var_os("XDG_DATA_DIRS")
            && !existing.is_empty()
        {
            data_dirs.push(";");
            data_dirs.push(existing);
        }
        set_process_env("XDG_DATA_DIRS", data_dirs);

        // Prevent loading incompatible host GTK4/GIO plugins from unrelated installs.
        set_process_env("GTK_PATH", root.join("lib").join("gtk-4.0"));
        set_process_env(
            "GIO_MODULE_DIR",
            root.join("lib").join("gio").join("modules"),
        );

        let pixbuf_root = root.join("lib").join("gdk-pixbuf-2.0");
        if let Some(abi_dir) = find_pixbuf_abi_dir(&pixbuf_root)
            && let Ok(template) = std::fs::read_to_string(abi_dir.join("loaders.cache.in"))
        {
            let loaders_dir = abi_dir.join("loaders");
            let rendered = render_loaders_cache(&template, &loaders_dir.to_string_lossy());
            let candidates = [
                std::env::var_os("LOCALAPPDATA")
                    .map(PathBuf::from)
                    .map(|p| p.join("simpad-control-panel-rs").join("loaders.cache")),
                Some(std::env::temp_dir().join("simpad-control-panel-rs-loaders.cache")),
            ];
            for cache_path in candidates.into_iter().flatten() {
                if let Some(parent) = cache_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if std::fs::write(&cache_path, rendered.as_bytes()).is_ok() {
                    set_process_env("GDK_PIXBUF_MODULEDIR", &loaders_dir);
                    set_process_env("GDK_PIXBUF_MODULE_FILE", &cache_path);
                    break;
                }
            }
        }
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    portable_env::configure_windows();

    let app = RelmApp::new("net.bysb.simpad.controlpanel");
    app.run::<AppModel>(());
}

#[cfg(test)]
mod tests {
    use super::portable_env::{portable_bundle_root, render_loaders_cache};
    use std::path::PathBuf;

    #[test]
    fn test_render_loaders_cache_escapes_windows_separators() {
        let template = "\"@LOADERS_DIR@\\\\libpixbufloader-svg.dll\"\n";
        let rendered = render_loaders_cache(
            template,
            r"C:\Program Files\SimPad Control Panel\lib\gdk-pixbuf-2.0\2.10.0\loaders\",
        );
        assert_eq!(
            rendered,
            "\"C:\\\\Program Files\\\\SimPad Control Panel\\\\lib\\\\gdk-pixbuf-2.0\\\\2.10.0\\\\loaders\\\\libpixbufloader-svg.dll\"\n"
        );
    }

    #[test]
    fn test_portable_bundle_root_returns_none_without_schemas() {
        let missing = PathBuf::from("/nonexistent-simpad-bundle-check");
        assert_eq!(portable_bundle_root(&missing), None);
    }
}
