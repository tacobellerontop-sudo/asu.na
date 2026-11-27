//! Windows registry access behind a trait so providers can be tested with a fake.

use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct RegKey {
    /// Full path with the long hive name, e.g. HKEY_LOCAL_MACHINE\SOFTWARE\...
    pub path: String,
    /// Last segment, e.g. a GOG game id.
    pub name: String,
    pub values: HashMap<String, String>,
}

impl RegKey {
    pub fn new(path: &str, values: &[(&str, &str)]) -> Self {
        Self {
            path: path.to_string(),
            name: path.rsplit('\\').next().unwrap_or(path).to_string(),
            values: values.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }
    }

    /// Value lookup that ignores case on the value name.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
            .filter(|v| !v.is_empty())
    }
}

pub trait Registry: Send + Sync {
    /// The key itself plus all of its subkeys (like `reg query <key> /s`). Empty when missing.
    fn query(&self, key: &str) -> Vec<RegKey>;
}

pub const UNINSTALL_KEYS: [&str; 3] = [
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
];

/// Lower-cases a key path and expands short hive names so paths compare equal.
pub fn normalize(path: &str) -> String {
    let lower = path.to_ascii_lowercase();
    let (hive, rest) = lower.split_once('\\').unwrap_or((&lower, ""));
    let hive = match hive {
        "hklm" => "hkey_local_machine",
        "hkcu" => "hkey_current_user",
        "hkcr" => "hkey_classes_root",
        "hku" => "hkey_users",
        h => h,
    };
    if rest.is_empty() {
        hive.to_string()
    } else {
        format!("{hive}\\{}", rest.trim_end_matches('\\'))
    }
}

/// Immediate children of `parent` in a recursive query result.
pub fn children<'a>(all: &'a [RegKey], parent: &str) -> Vec<&'a RegKey> {
    let prefix = format!("{}\\", normalize(parent));
    all.iter()
        .filter(|k| {
            let p = normalize(&k.path);
            p.starts_with(&prefix) && !p[prefix.len()..].contains('\\')
        })
        .collect()
}

/// Splits `"C:\Game Dir\unins000.exe" /SILENT` into file and args.
pub fn split_command_line(cmd: &str) -> Option<(String, Vec<String>)> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for ch in cmd.chars() {
        match ch {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    parts.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        parts.push(cur);
    }
    // Unquoted path with spaces: grow the file part until it ends in .exe
    if !cmd.starts_with('"') && !parts[0].to_ascii_lowercase().ends_with(".exe") {
        if let Some(i) = parts.iter().position(|p| p.to_ascii_lowercase().ends_with(".exe")) {
            return Some((parts[..=i].join(" "), parts[i + 1..].to_vec()));
        }
    }
    let file = parts.remove(0);
    Some((file, parts))
}

/// The real registry. Not available off Windows.
pub struct SystemRegistry;

#[cfg(not(windows))]
impl Registry for SystemRegistry {
    fn query(&self, _key: &str) -> Vec<RegKey> {
        Vec::new()
    }
}

#[cfg(windows)]
impl Registry for SystemRegistry {
    fn query(&self, key: &str) -> Vec<RegKey> {
        use winreg::enums::*;
        let norm = normalize(key);
        let (hive_name, sub) = norm.split_once('\\').unwrap_or((&norm, ""));
        let (hive, long) = match hive_name {
            "hkey_local_machine" => (HKEY_LOCAL_MACHINE, "HKEY_LOCAL_MACHINE"),
            "hkey_current_user" => (HKEY_CURRENT_USER, "HKEY_CURRENT_USER"),
            "hkey_classes_root" => (HKEY_CLASSES_ROOT, "HKEY_CLASSES_ROOT"),
            _ => return Vec::new(),
        };
        // Keep the original casing of the sub path for display.
        let sub_orig = key.split_once('\\').map(|(_, s)| s).unwrap_or("");
        let root = winreg::RegKey::predef(hive);
        let mut out = Vec::new();
        if let Ok(k) = root.open_subkey_with_flags(sub_orig, KEY_READ) {
            walk(&k, &format!("{long}\\{sub_orig}"), 0, &mut out);
        }
        let _ = sub;
        out
    }
}

#[cfg(windows)]
fn walk(key: &winreg::RegKey, path: &str, depth: usize, out: &mut Vec<RegKey>) {
    use winreg::enums::*;
    let mut values = HashMap::new();
    for (name, value) in key.enum_values().flatten() {
        let s = match value.vtype {
            REG_DWORD => key.get_value::<u32, _>(&name).map(|v| v.to_string()).unwrap_or_default(),
            REG_QWORD => key.get_value::<u64, _>(&name).map(|v| v.to_string()).unwrap_or_default(),
            _ => key.get_value::<String, _>(&name).unwrap_or_default(),
        };
        values.insert(name, s);
    }
    out.push(RegKey {
        path: path.to_string(),
        name: path.rsplit('\\').next().unwrap_or(path).to_string(),
        values,
    });
    if depth >= 4 {
        return;
    }
    for name in key.enum_keys().flatten() {
        if let Ok(child) = key.open_subkey_with_flags(&name, KEY_READ) {
            walk(&child, &format!("{path}\\{name}"), depth + 1, out);
        }
    }
}

/// In-memory registry for tests.
#[derive(Default)]
pub struct FakeRegistry(pub Vec<RegKey>);

impl Registry for FakeRegistry {
    fn query(&self, key: &str) -> Vec<RegKey> {
        let q = normalize(key);
        self.0
            .iter()
            .filter(|k| {
                let p = normalize(&k.path);
                p == q || p.starts_with(&format!("{q}\\"))
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn children_across_hive_spellings() {
        let r = FakeRegistry(vec![
            RegKey::new(r"HKEY_LOCAL_MACHINE\SOFTWARE\GOG.com\Games\1", &[("gameName", "A")]),
            RegKey::new(r"HKEY_LOCAL_MACHINE\SOFTWARE\GOG.com\Games\1\sub", &[]),
        ]);
        let all = r.query(r"HKLM\SOFTWARE\GOG.com\Games");
        assert_eq!(all.len(), 2);
        let kids = children(&all, r"HKLM\SOFTWARE\GOG.com\Games");
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].get("GAMENAME"), Some("A"));
    }

    #[test]
    fn splits_command_lines() {
        assert_eq!(
            split_command_line(r#""C:\Game Dir\unins000.exe" /SILENT"#),
            Some((r"C:\Game Dir\unins000.exe".into(), vec!["/SILENT".into()]))
        );
        assert_eq!(
            split_command_line(r"C:\Program Files\Game\uninstall.exe /x"),
            Some((r"C:\Program Files\Game\uninstall.exe".into(), vec!["/x".into()]))
        );
        assert_eq!(split_command_line("  "), None);
    }
}
