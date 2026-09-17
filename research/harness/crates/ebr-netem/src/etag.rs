//! ETag formatting and `If-Match` evaluation.
//!
//! The exact contract `entrybound::random_access::HttpRangeSource` requires
//! is read directly off `crates/entrybound/src/random_access.rs`
//! (`parse_strong_etag`, `HttpRangeSource::read_exact_at`): a *strong*,
//! quoted, opaque-tag-syntax ETag (`"<etagc>*"`, never `W/"..."'`), sent back
//! unchanged on every ranged `GET`, with the client sending
//! `If-Match: "<etag>"` on every one of those `GET`s and treating a `412`
//! response as "the source changed underneath me, abort". `ebr-origin`'s
//! `--etag` toggle exists to deliberately violate that contract (weak or
//! missing) so an experiment can confirm a client actually refuses it,
//! exactly as `crates/entrybound`'s own unit tests already do against a
//! hand-rolled TCP fixture.

/// `--etag` modes `ebr-origin` accepts. `Weak` and `Missing` exist to be
/// deliberately non-conformant -- see this module's doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtagMode {
    Strong,
    Weak,
    Missing,
}

impl EtagMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "strong" => Some(EtagMode::Strong),
            "weak" => Some(EtagMode::Weak),
            "missing" => Some(EtagMode::Missing),
            _ => None,
        }
    }
}

/// Formats `content_hash_hex` (lower-case hex, e.g. from
/// [`ebr_common::hash::sha256_hex`]) as an `ETag` field-value per `mode`, or
/// `None` when `mode` is [`EtagMode::Missing`] (the header should be
/// omitted entirely).
pub fn format_etag(content_hash_hex: &str, mode: EtagMode) -> Option<String> {
    match mode {
        EtagMode::Strong => Some(format!("\"{content_hash_hex}\"")),
        EtagMode::Weak => Some(format!("W/\"{content_hash_hex}\"")),
        EtagMode::Missing => None,
    }
}

/// Evaluates an `If-Match` request header (RFC 7232 §3.1) against the
/// resource's *current* strong ETag (already-quoted, e.g. `"abcd"`), using
/// strong comparison (an ETag from a weak/missing-mode server never
/// satisfies an explicit `If-Match` list, matching the RFC: strong
/// comparison requires both sides to be strong validators).
///
/// - No `If-Match` header: precondition trivially holds.
/// - `If-Match: *`: holds as long as the resource exists (this function is
///   only ever called once the requested file is known to exist).
/// - Otherwise: holds iff `current_strong_etag` is `Some` and appears
///   verbatim (including quotes) in the comma-separated list.
pub fn if_match_satisfied(
    if_match_header: Option<&str>,
    current_strong_etag: Option<&str>,
) -> bool {
    let Some(header) = if_match_header else {
        return true;
    };
    let header = header.trim();
    if header == "*" {
        return true;
    }
    let Some(current) = current_strong_etag else {
        return false;
    };
    header.split(',').any(|tag| tag.trim() == current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_mode_quotes_the_hash() {
        assert_eq!(
            format_etag("abcd1234", EtagMode::Strong),
            Some("\"abcd1234\"".to_string())
        );
    }

    #[test]
    fn weak_mode_prefixes_w_slash() {
        assert_eq!(
            format_etag("abcd1234", EtagMode::Weak),
            Some("W/\"abcd1234\"".to_string())
        );
    }

    #[test]
    fn missing_mode_omits_the_header() {
        assert_eq!(format_etag("abcd1234", EtagMode::Missing), None);
    }

    #[test]
    fn parses_the_three_named_modes_and_rejects_anything_else() {
        assert_eq!(EtagMode::parse("strong"), Some(EtagMode::Strong));
        assert_eq!(EtagMode::parse("weak"), Some(EtagMode::Weak));
        assert_eq!(EtagMode::parse("missing"), Some(EtagMode::Missing));
        assert_eq!(EtagMode::parse("bogus"), None);
    }

    #[test]
    fn no_if_match_header_always_satisfies() {
        assert!(if_match_satisfied(None, Some("\"a\"")));
        assert!(if_match_satisfied(None, None));
    }

    #[test]
    fn star_satisfies_regardless_of_etag() {
        assert!(if_match_satisfied(Some("*"), Some("\"a\"")));
        assert!(if_match_satisfied(Some("*"), None));
    }

    #[test]
    fn matches_only_the_exact_quoted_tag() {
        assert!(if_match_satisfied(Some("\"a\""), Some("\"a\"")));
        assert!(!if_match_satisfied(Some("\"a\""), Some("\"b\"")));
        assert!(if_match_satisfied(Some("\"a\", \"b\""), Some("\"b\"")));
    }

    #[test]
    fn never_matches_when_the_server_has_no_strong_etag() {
        assert!(!if_match_satisfied(Some("\"a\""), None));
    }
}
