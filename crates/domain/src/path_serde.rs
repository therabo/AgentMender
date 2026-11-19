//! Preserve native paths losslessly in JSON. UTF-8 paths remain ordinary strings.
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Representation {
    Text(String),
    Native { encoding: String, hex: String },
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 15) as usize] as char);
    }
    result
}
fn unhex(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|part| {
            let high = (part[0] as char).to_digit(16)?;
            let low = (part[1] as char).to_digit(16)?;
            Some(((high << 4) | low) as u8)
        })
        .collect()
}
fn represent(path: &Path) -> Representation {
    if let Some(text) = path.to_str() {
        return Representation::Text(text.into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Representation::Native {
            encoding: "unix_bytes".into(),
            hex: hex(path.as_os_str().as_bytes()),
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let bytes: Vec<_> = path
            .as_os_str()
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .collect();
        Representation::Native {
            encoding: "windows_utf16le".into(),
            hex: hex(&bytes),
        }
    }
    #[cfg(not(any(unix, windows)))]
    Representation::Native {
        encoding: "unsupported_native_path".into(),
        hex: hex(path.as_os_str().as_encoded_bytes()),
    }
}
fn restore<E: Error>(value: Representation) -> Result<PathBuf, E> {
    let (encoding, hex) = match value {
        Representation::Text(text) => return Ok(text.into()),
        Representation::Native { encoding, hex } => (encoding, hex),
    };
    let bytes = unhex(&hex).ok_or_else(|| E::custom("invalid native path encoding"))?;
    #[cfg(unix)]
    if encoding == "unix_bytes" {
        use std::os::unix::ffi::OsStringExt;
        return Ok(OsString::from_vec(bytes).into());
    }
    #[cfg(windows)]
    if encoding == "windows_utf16le" && bytes.len().is_multiple_of(2) {
        use std::os::windows::ffi::OsStringExt;
        let words: Vec<_> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        return Ok(OsString::from_wide(&words).into());
    }
    Err(E::custom(
        "native path encoding does not match this platform",
    ))
}
pub fn serialize<S: Serializer>(path: &Path, serializer: S) -> Result<S::Ok, S::Error> {
    represent(path).serialize(serializer)
}
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
    restore(Representation::deserialize(deserializer)?)
}
