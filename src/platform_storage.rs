// Platform-agnostic small-string persistence.
//
// All persistent settings (config.json, save slots, paint, spawn points,
// keybindings) used to write directly to ~/.skoffroad/<file> via std::fs.
// That doesn't work in browsers (WASM has no filesystem) and forced every
// caller to know the home-dir convention.
//
// This module hides the difference behind three calls:
//
//     read_string(key)  -> Option<String>
//     write_string(key, content) -> Result<(), String>
//     exists(key)       -> bool
//
// `key` is a short relative path like "config.json" or "save_slot_1.json".
// Native: maps to $HOME/.skoffroad/<key>; WASM: localStorage["skoffroad/<key>"].

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::fs;
    use std::io::Write as IoWrite;
    use std::path::PathBuf;

    fn root_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let mut p = PathBuf::from(home);
        p.push(".skoffroad");
        p
    }

    pub fn key_path(key: &str) -> PathBuf {
        let mut p = root_dir();
        p.push(key);
        p
    }

    pub fn read_string(key: &str) -> Option<String> {
        fs::read_to_string(key_path(key)).ok()
    }

    pub fn write_string(key: &str, content: &str) -> Result<(), String> {
        let path = key_path(key);
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(format!(
                    "could not create directory {}: {}",
                    parent.display(),
                    e
                ));
            }
        }
        let mut file = fs::File::create(&path)
            .map_err(|e| format!("could not open {} for writing: {}", path.display(), e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("write failed for {}: {}", path.display(), e))?;
        Ok(())
    }

    pub fn exists(key: &str) -> bool {
        key_path(key).exists()
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    /// All keys are written under this prefix so we don't collide with other
    /// origins / web apps sharing the localStorage namespace.
    const PREFIX: &str = "skoffroad/";

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    pub fn read_string(key: &str) -> Option<String> {
        storage()?.get_item(&format!("{}{}", PREFIX, key)).ok().flatten()
    }

    pub fn write_string(key: &str, content: &str) -> Result<(), String> {
        let s = storage().ok_or_else(|| "no localStorage available".to_string())?;
        s.set_item(&format!("{}{}", PREFIX, key), content)
            .map_err(|_| format!("localStorage.setItem failed for {}", key))
    }

    pub fn exists(key: &str) -> bool {
        match storage() {
            Some(s) => s.get_item(&format!("{}{}", PREFIX, key)).ok().flatten().is_some(),
            None => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API — same surface on both platforms
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
pub use native::{exists, read_string, write_string};

#[cfg(target_arch = "wasm32")]
pub use wasm::{exists, read_string, write_string};

// ---------------------------------------------------------------------------
// Wall-clock helpers
// ---------------------------------------------------------------------------
//
// `std::time::SystemTime::now()` panics on wasm32-unknown-unknown
// ("time not implemented on this platform"). The browser equivalent is
// `Date.now()`. This helper hides the difference and is safe to call from
// any plugin Startup / Update system.

/// Seconds since the Unix epoch. Falls back to 0 if the clock is unavailable.
pub fn epoch_seconds() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
    #[cfg(target_arch = "wasm32")]
    {
        // `Date.now()` returns milliseconds since epoch as f64.
        let ms = js_sys_date_now();
        if ms.is_finite() && ms >= 0.0 {
            (ms / 1000.0) as u64
        } else {
            0
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn js_sys_date_now() -> f64 {
    // Inline binding to JS `Date.now()` so we don't have to pull in `js-sys`
    // as a dependency. `web-sys` already ships with the wasm-bindgen runtime.
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = Date)]
        fn now() -> f64;
    }
    now()
}

/// Native-only helper: returns the on-disk path for a key. Useful for log
/// messages ("config: loaded from /home/.../config.json"). Returns `None`
/// in WASM where the concept doesn't apply.
#[cfg(not(target_arch = "wasm32"))]
pub fn debug_path(key: &str) -> Option<std::path::PathBuf> {
    Some(native::key_path(key))
}

#[cfg(target_arch = "wasm32")]
pub fn debug_path(_key: &str) -> Option<std::path::PathBuf> {
    None
}

// ---------------------------------------------------------------------------
// Tests (native only)
// ---------------------------------------------------------------------------

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;

    #[test]
    fn round_trip_in_temp() {
        // Use a junk key under a temp HOME so we don't touch real config.
        let tmp = std::env::temp_dir().join("skoffroad_test_home");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        // Override $HOME for this test thread. Note: `std::env::set_var` is
        // unsafe-shared across threads but Cargo runs unit tests serially
        // unless --test-threads > 1, and this test doesn't read HOME from
        // any other test.
        unsafe {
            std::env::set_var("HOME", &tmp);
        }

        assert!(!exists("foo.json"));
        assert!(read_string("foo.json").is_none());

        write_string("foo.json", "hello").unwrap();
        assert!(exists("foo.json"));
        assert_eq!(read_string("foo.json").as_deref(), Some("hello"));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
