//! Convenience builders for Inertia using [vitejs].
//!
//! This module provides [Development] and [Production] structs for
//! different environments, e.g.:
//!
//! ```rust
//! use axum_inertia::vite;
//!
//! enum Env {
//!   Dev,
//!   Prod,
//! }
//!
//! let env = match std::env::var("APP_ENV").map_or(false, |s| &s[..] == "production") {
//!     true => Env::Prod,
//!     false => Env::Dev
//! };
//!
//! let inertia = match env {
//!     Env::Dev => vite::Development::default()
//!         .port(5173)
//!         .main("src/main.ts")
//!         .lang("en")
//!         .title("My app")
//!         .into_inertia(),
//!     Env::Prod => vite::Production::new("client/dist/manifest.json", "src/main.ts")
//!         .unwrap()
//!         .lang("en")
//!         .title("My app")
//!         .into_inertia(),
//! };
//! ```
//!
//! [vitejs]: https://vitejs.dev
use crate::Inertia;
use hex::encode;
use maud::{html, PreEscaped};
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::collections::HashMap;

pub struct Development {
    port: u16,
    main: &'static str,
    lang: &'static str,
    title: &'static str,
}

impl Default for Development {
    fn default() -> Self {
        Development {
            port: 5173,
            main: "src/main.ts",
            lang: "en",
            title: "Vite",
        }
    }
}

impl Development {
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn main(mut self, main: &'static str) -> Self {
        self.main = main;
        self
    }

    pub fn lang(mut self, lang: &'static str) -> Self {
        self.lang = lang;
        self
    }

    pub fn title(mut self, title: &'static str) -> Self {
        self.title = title;
        self
    }

    pub fn into_inertia(self) -> Inertia {
        let layout = Box::new(move |props| {
            let vite_src = format!("http://localhost:{}/@vite/client", self.port);
            let main_src = format!("http://localhost:{}/{}", self.port, self.main);
            html! {
                html lang=(self.lang) {
                    head {
                        title { (self.title) }
                        meta charset="utf-8";
                        meta name="viewport" content="width=device-width, initial-scale=1.0";
                        script type="module" src=(vite_src) {}
                        script type="module" src=(main_src) {}
                    }
                    body {
                        div #app data-page=(props) {}
                    }
                }
            }
            .into_string()
        });
        Inertia::new(None, layout)
    }
}

pub struct Production {
    main: String,
    css: Option<String>,
    title: &'static str,
    lang: &'static str,
    /// SHA1 hash of the contents of the manifest file.
    version: String,
}

impl Production {
    pub fn new(
        manifest_path: &'static str,
        main: &'static str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let bytes = std::fs::read(manifest_path)?;
        let manifest: HashMap<String, ManifestEntry> =
            serde_json::from_str(&String::from_utf8(bytes.clone())?)?;
        let entry = manifest.get(main).ok_or(ViteError::EntryMissing(main))?;
        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        let result = hasher.finalize();
        let version = encode(result);
        let css = {
            if let Some(css_sources) = &entry.css {
                let mut css = String::new();
                for source in css_sources {
                    css.push_str(&format!(r#"<link rel="stylesheet" href="/{source}"/>"#));
                }
                Some(css)
            } else {
                None
            }
        };
        Ok(Self {
            main: format!("/{}", entry.file),
            css,
            title: "Vite",
            lang: "en",
            version,
        })
    }

    pub fn lang(mut self, lang: &'static str) -> Self {
        self.lang = lang;
        self
    }

    pub fn title(mut self, title: &'static str) -> Self {
        self.title = title;
        self
    }

    pub fn into_inertia(self) -> Inertia {
        let layout = Box::new(move |props| {
            let css = self.css.clone().unwrap_or("".to_string());
            html! {
                html lang=(self.lang) {
                    head {
                        title { (self.title) }
                        meta charset="utf-8";
                        meta name="viewport" content="width=device-width, initial-scale=1.0";
                        script type="module" src=(self.main) {}
                        (PreEscaped(css))
                    }
                    body {
                        div #app data-page=(props) {}
                    }
                }
            }
            .into_string()
        });
        Inertia::new(Some(self.version), layout)
    }
}

#[derive(Debug)]
pub enum ViteError {
    ManifestMissing(std::io::Error),
    EntryMissing(&'static str),
}

impl std::fmt::Display for ViteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ManifestMissing(_) => write!(f, "couldn't open manifest file"),
            Self::EntryMissing(entry) => write!(f, "manifest missing entry for {}", entry),
        }
    }
}

impl std::error::Error for ViteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ManifestMissing(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    file: String,
    css: Option<Vec<String>>,
}
