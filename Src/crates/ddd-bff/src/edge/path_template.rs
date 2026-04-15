//! Path template helpers.
//!
//! Our YAML templates use `{name}` for a single-segment capture and
//! `{**name}` for a catch-all. [`matchit`] uses `{name}` and `{*name}`
//! respectively, so we translate between them here.

/// Convert a YAML path template to the syntax accepted by
/// [`matchit::Router`].
///
/// - `{name}` → `{name}` (unchanged)
/// - `{**name}` → `{*name}`
pub fn to_matchit(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let bytes = path.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let close = match memchr::memchr(b'}', &bytes[i..]) {
                Some(j) => i + j,
                None => {
                    out.push_str(&path[i..]);
                    break;
                }
            };
            let inside = &path[i + 1..close];
            let translated = match inside.strip_prefix("**") {
                Some(rest) => format!("{{*{rest}}}"),
                None => format!("{{{inside}}}"),
            };
            out.push_str(&translated);
            i = close + 1;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// Extract parameter names (without the `*` / `**` prefix) from a YAML
/// template path, in declaration order.
pub fn extract_params(path: &str) -> Vec<String> {
    let mut params = Vec::new();
    let bytes = path.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let close = match memchr::memchr(b'}', &bytes[i..]) {
                Some(j) => i + j,
                None => break,
            };
            let name = path[i + 1..close]
                .trim_start_matches('*')
                .trim_start_matches('*');
            if !name.is_empty() {
                params.push(name.to_owned());
            }
            i = close + 1;
        } else {
            i += 1;
        }
    }
    params
}

// Lightweight dependency-free memchr fallback used above.
mod memchr {
    pub fn memchr(needle: u8, haystack: &[u8]) -> Option<usize> {
        haystack.iter().position(|b| *b == needle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_path_unchanged() {
        assert_eq!(to_matchit("/api/orders"), "/api/orders");
        assert_eq!(to_matchit("/health"), "/health");
    }

    #[test]
    fn single_segment_param_unchanged() {
        assert_eq!(to_matchit("/api/orders/{id}"), "/api/orders/{id}");
        assert_eq!(
            to_matchit("/a/{x}/b/{y}"),
            "/a/{x}/b/{y}"
        );
    }

    #[test]
    fn catchall_translated() {
        assert_eq!(to_matchit("/static/{**rest}"), "/static/{*rest}");
    }

    #[test]
    fn extract_params_basic() {
        assert_eq!(
            extract_params("/api/orders/{id}"),
            vec!["id".to_owned()]
        );
        assert_eq!(
            extract_params("/a/{x}/b/{y}"),
            vec!["x".to_owned(), "y".to_owned()]
        );
    }

    #[test]
    fn extract_params_catchall() {
        assert_eq!(
            extract_params("/static/{**rest}"),
            vec!["rest".to_owned()]
        );
    }

    #[test]
    fn extract_params_none() {
        assert!(extract_params("/api/orders").is_empty());
    }
}
