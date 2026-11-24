use std::{ffi::OsString, os::unix::ffi::OsStringExt, path::PathBuf};

pub(super) fn paths(data: &str) -> Vec<PathBuf> {
    data.lines()
        .filter_map(|line| {
            let line = line.trim_end_matches('\r');
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            if let Some(uri) = line.strip_prefix("file://") {
                let path = if uri.starts_with('/') {
                    uri
                } else if uri.starts_with("localhost/") {
                    &uri["localhost".len()..]
                } else {
                    eprintln!("Ignoring non-local dropped file URI: {line}");
                    return None;
                };
                let bytes = path.as_bytes();
                let mut decoded = Vec::with_capacity(bytes.len());
                let mut index = 0;
                while index < bytes.len() {
                    if bytes[index] == b'%' {
                        let hex = bytes.get(index + 1..index + 3);
                        let value = hex.and_then(|hex| {
                            let digit = |byte: u8| (byte as char).to_digit(16);
                            Some((digit(hex[0])? * 16 + digit(hex[1])?) as u8)
                        });
                        match value {
                            Some(value) if value != 0 => decoded.push(value),
                            _ => {
                                eprintln!("Ignoring malformed dropped file URI: {line}");
                                return None;
                            }
                        }
                        index += 3;
                    } else {
                        decoded.push(bytes[index]);
                        index += 1;
                    }
                }
                Some(PathBuf::from(OsString::from_vec(decoded)))
            } else if line.starts_with('/') {
                Some(PathBuf::from(line))
            } else {
                eprintln!("Ignoring unsupported dropped file path: {line}");
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_local_uri_lists_and_plain_paths() {
        assert_eq!(
            paths("# comment\r\nfile:///tmp/a%20b.P\r\nfile://localhost/tmp/c.P\n/tmp/d.P\nfile://other/tmp/e.P\n"),
            vec![
                PathBuf::from("/tmp/a b.P"),
                PathBuf::from("/tmp/c.P"),
                PathBuf::from("/tmp/d.P")
            ]
        );
        assert!(paths("file:///tmp/%xx\nfile:///tmp/%00\n").is_empty());
    }
}
