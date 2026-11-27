//! Minimal parser for Valve's text KeyValues format (.vdf / .acf).
//! Steam is inconsistent about key casing, so lookups are case-insensitive.

#[derive(Clone, Debug, PartialEq)]
pub enum Vdf {
    Str(String),
    Obj(Vec<(String, Vdf)>),
}

impl Vdf {
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Obj(items) => items.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v),
            Vdf::Str(_) => None,
        }
    }

    pub fn path(&self, keys: &[&str]) -> Option<&Vdf> {
        keys.iter().try_fold(self, |cur, k| cur.get(k))
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            Vdf::Obj(_) => None,
        }
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Vdf::str)
    }

    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Obj(items) => items,
            Vdf::Str(_) => &[],
        }
    }
}

pub fn parse(text: &str) -> Vdf {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    parse_obj(&chars, &mut i)
}

fn skip(c: &[char], i: &mut usize) {
    while *i < c.len() {
        match c[*i] {
            ' ' | '\t' | '\r' | '\n' | '\u{feff}' => *i += 1,
            '/' if c.get(*i + 1) == Some(&'/') => {
                while *i < c.len() && c[*i] != '\n' {
                    *i += 1
                }
            }
            '[' => {
                while *i < c.len() && c[*i] != ']' {
                    *i += 1
                }
                *i += 1
            }
            _ => break,
        }
    }
}

enum Tok {
    Open,
    Close,
    Str(String),
}

fn token(c: &[char], i: &mut usize) -> Option<Tok> {
    skip(c, i);
    let ch = *c.get(*i)?;
    match ch {
        '{' => {
            *i += 1;
            Some(Tok::Open)
        }
        '}' => {
            *i += 1;
            Some(Tok::Close)
        }
        '"' => {
            *i += 1;
            let mut s = String::new();
            while *i < c.len() && c[*i] != '"' {
                if c[*i] == '\\' && *i + 1 < c.len() {
                    s.push(match c[*i + 1] {
                        'n' => '\n',
                        't' => '\t',
                        e => e,
                    });
                    *i += 2;
                } else {
                    s.push(c[*i]);
                    *i += 1;
                }
            }
            *i += 1;
            Some(Tok::Str(s))
        }
        _ => {
            let mut s = String::new();
            while *i < c.len() && !c[*i].is_whitespace() && !matches!(c[*i], '{' | '}' | '"') {
                s.push(c[*i]);
                *i += 1;
            }
            Some(Tok::Str(s))
        }
    }
}

fn parse_obj(c: &[char], i: &mut usize) -> Vdf {
    let mut items = Vec::new();
    loop {
        let key = match token(c, i) {
            Some(Tok::Str(s)) => s,
            _ => return Vdf::Obj(items),
        };
        let save = *i;
        match token(c, i) {
            Some(Tok::Open) => items.push((key, parse_obj(c, i))),
            Some(Tok::Str(v)) => items.push((key, Vdf::Str(v))),
            Some(Tok::Close) => {
                items.push((key, Vdf::Str(String::new())));
                return Vdf::Obj(items);
            }
            None => {
                *i = save;
                items.push((key, Vdf::Str(String::new())));
                return Vdf::Obj(items);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_objects_escapes_and_comments() {
        let v = parse(
            r#"// comment
"Root"
{
  "path"   "C:\\Games\\Steam"
  "Nested" { "a" "1" "empty" "" }
  bare value
}"#,
        );
        assert_eq!(v.path(&["root", "PATH"]).and_then(Vdf::str), Some(r"C:\Games\Steam"));
        assert_eq!(v.path(&["Root", "nested", "A"]).and_then(Vdf::str), Some("1"));
        assert_eq!(v.path(&["Root", "nested", "empty"]).and_then(Vdf::str), Some(""));
        assert_eq!(v.path(&["Root", "bare"]).and_then(Vdf::str), Some("value"));
    }
}
