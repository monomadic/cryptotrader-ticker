#![allow(dead_code)]
#![allow(unused_variables)]

use dirs;
use serde_derive::Deserialize;
use std::collections::BTreeMap;
use toml;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub exchange: BTreeMap<String, BTreeMap<String, Pair>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Pair {
    pub base: String,
    pub entry_price: Option<f64>,
}

// converted a lot of this to use unwraps because... well.. rust gets so exhausting
// at times and this is quick code, I didn't want an error object in it.

pub fn read() -> Result<Config, String> {
    pub fn file_exists(path: &str) -> bool {
        use std::fs;

        match fs::metadata(path) {
            Ok(p) => p.is_file(),
            Err(_) => false,
        }
    }

    fn str_from_file_path(path: &str) -> Result<String, String> {
        use std::io::prelude::*;

        let mut handle = ::std::fs::File::open(path).unwrap();
        let mut bytebuffer = Vec::new();

        handle.read_to_end(&mut bytebuffer).unwrap();

        Ok(String::from_utf8(bytebuffer).unwrap())
    }

    let home_path = dirs::home_dir().unwrap();

    // search paths for config files, in order of search preference.
    let search_paths = vec![
        format!("./ticker.toml"),
        format!("{}/.ticker.toml", home_path.display()),
        format!("{}/.crypto/ticker.toml", home_path.display()),
    ];

    for path in search_paths.clone() {
        if file_exists(&path) {
            return Ok(toml::from_str(&str_from_file_path(&path).unwrap()).unwrap());
        }
    }

    Err("error loading config.".to_string())
}
