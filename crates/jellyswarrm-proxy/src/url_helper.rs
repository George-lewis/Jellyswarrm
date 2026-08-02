use url::Url;
use uuid::Uuid;

pub fn is_id_like(segment: &str) -> bool {
    Uuid::parse_str(segment).is_ok()
}

/// Joins a server URL with a request path, preserving any subdirectories in the server URL
///
/// # Examples
///
/// ```
/// use url::Url;
/// let server_url = Url::parse("http://server.com/jellyfin").unwrap();
/// let request_path = "/Users/123";
/// let result = join_server_url(&server_url, request_path);
/// assert_eq!(result.as_str(), "http://server.com/jellyfin/Users/123");
/// ```
pub fn join_server_url(server_url: &Url, request_path: &str) -> Url {
    let mut new_url = server_url.clone();
    let server_path = new_url.path().trim_end_matches('/');
    let combined_path = if server_path.is_empty() {
        request_path.to_string()
    } else {
        format!("{}{}", server_path, request_path)
    };
    new_url.set_path(&combined_path);
    new_url
}

pub fn contains_id(url: &Url, name: &str) -> Option<String> {
    contains_id_at_offset(url, name, 1)
}

/// Finds an ID-like path segment `offset` segments after the segment matching
/// `name`.
///
/// Most Jellyfin routes put the ID directly after a literal tag
/// (`/Items/{itemId}`), which is `offset == 1`. Some plugin routes address a
/// resource by two IDs at once -- Jellyfin Enhanced uses
/// `/JellyfinEnhanced/watch-progress/{userId}/{itemId}` -- so the item ID sits
/// at `offset == 2` with the user ID in between. A plain "tag then ID" match
/// cannot reach it, because the segment before it is itself an ID rather than a
/// literal.
pub fn contains_id_at_offset(url: &Url, name: &str, offset: usize) -> Option<String> {
    let segments: Vec<&str> = match url.path_segments() {
        Some(segments) => segments.collect(),
        None => Vec::new(),
    };

    for i in 0..segments.len() {
        let Some(candidate_index) = i.checked_add(offset) else {
            continue;
        };
        if candidate_index >= segments.len() {
            continue;
        }

        if segments[i].eq_ignore_ascii_case(name) && is_id_like(segments[candidate_index]) {
            return Some(segments[candidate_index].to_string());
        }
    }
    None
}

pub fn replace_id(url: Url, original: &str, replacement: &str) -> Url {
    let mut url = url;
    let Some(segments) = url.path_segments() else {
        return url;
    };

    let replaced_segments = segments
        .map(|segment| {
            if segment == original {
                replacement
            } else {
                segment
            }
        })
        .collect::<Vec<_>>();

    url.set_path(&replaced_segments.join("/"));
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_server_url() {
        // Test with server having subdirectory
        let server_url = Url::parse("http://server.com/jellyfin").unwrap();
        let result = join_server_url(&server_url, "/Users/123");
        assert_eq!(result.as_str(), "http://server.com/jellyfin/Users/123");

        // Test with server at root
        let server_url = Url::parse("http://server.com").unwrap();
        let result = join_server_url(&server_url, "/Users/123");
        assert_eq!(result.as_str(), "http://server.com/Users/123");

        // Test with server having trailing slash
        let server_url = Url::parse("http://server.com/jellyfin/").unwrap();
        let result = join_server_url(&server_url, "/Users/123");
        assert_eq!(result.as_str(), "http://server.com/jellyfin/Users/123");
    }

    #[test]
    fn test_is_id_like() {
        assert!(is_id_like("0123456789abcdef0123456789abcdef"));
        assert!(is_id_like("c3256b7a-96f3-4772-b7d5-cacb090bbb02")); // with dashes
        assert!(!is_id_like("0123456789abcdef0123456789abcde")); // 31 chars
        assert!(!is_id_like("g123456789abcdef0123456789abcdef")); // non-hex
    }

    #[test]
    fn test_contains_id_found() {
        let url =
            Url::parse("https://example.com/foo/0123456789abcdef0123456789abcdef/bar").unwrap();
        assert_eq!(
            contains_id(&url, "foo"),
            Some("0123456789abcdef0123456789abcdef".to_string())
        );
    }

    #[test]
    fn test_contains_id_not_found() {
        let url = Url::parse("https://example.com/foo/bar").unwrap();
        assert_eq!(contains_id(&url, "foo"), None);
    }

    #[test]
    fn test_contains_id_at_offset() {
        // /JellyfinEnhanced/watch-progress/{userId}/{itemId}
        let url = Url::parse(
            "https://example.com/JellyfinEnhanced/watch-progress/\
             8502fd20-3583-4ea0-b058-ed4b4fc78e7b/0ac31223-37b9-4e40-bf48-79c473d07aca",
        )
        .unwrap();

        assert_eq!(
            contains_id_at_offset(&url, "watch-progress", 1),
            Some("8502fd20-3583-4ea0-b058-ed4b4fc78e7b".to_string())
        );
        assert_eq!(
            contains_id_at_offset(&url, "watch-progress", 2),
            Some("0ac31223-37b9-4e40-bf48-79c473d07aca".to_string())
        );
        // Offset running past the end of the path must not panic.
        assert_eq!(contains_id_at_offset(&url, "watch-progress", 3), None);
    }

    #[test]
    fn test_replace_id() {
        let url =
            Url::parse("https://example.com/foo/0123456789abcdef0123456789abcdef/bar").unwrap();
        let replaced = replace_id(
            url,
            "0123456789abcdef0123456789abcdef",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        assert_eq!(replaced.path(), "/foo/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/bar");
    }
}
