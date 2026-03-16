use std::{collections::HashMap, sync::OnceLock};

const STRINGS_SRC: &str = include_str!("strings.properties");

static STRINGS: OnceLock<Strings> = OnceLock::new();

pub struct Strings {
    map: HashMap<String, String>,
}

impl Strings {
    pub fn get() -> &'static Self {
        STRINGS.get_or_init(|| {
            let map = STRINGS_SRC
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    !trimmed.is_empty() && !trimmed.starts_with('#')
                })
                .filter_map(|line| {
                    let (key, value) = line.split_once('=')?;
                    let key = key.trim().to_string();
                    let value = value.strip_prefix(' ').unwrap_or(value).to_string();
                    Some((key, value))
                })
                .collect();
            Self { map }
        })
    }

    // Returns the raw string for a key, or the key itself if not found.
    pub fn raw<'a>(&'a self, key: &'a str) -> &'a str {
        self.map.get(key).map_or(key, std::string::String::as_str)
    }

    // Substitutes {0}, {1}, {2}... with the provided args.
    pub fn fmt(&self, key: &str, args: &[&str]) -> String {
        let mut result = self.raw(key).to_string();
        for (i, arg) in args.iter().enumerate() {
            result = result.replace(&format!("{{{i}}}"), arg);
        }
        result
    }
}
