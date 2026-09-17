//! `Range: bytes=...` request-header parsing and `multipart/byteranges`
//! response framing (RFC 7233 §2.1, §4.1), independent of any HTTP library
//! so it is unit-testable without a running server.

/// One resolved, inclusive byte range against a known total length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub start: u64,
    /// Inclusive.
    pub end: u64,
}

impl ByteRange {
    /// Always at least 1: a resolved `ByteRange` covers at least one byte.
    pub fn len(&self) -> u64 {
        self.end - self.start + 1
    }

    /// Always `false`: a resolved `ByteRange` always covers at least one
    /// byte (`start <= end`, enforced by [`crate::range::parse_range_header`]).
    /// Exists only so clippy's `len_without_is_empty` does not flag `len`.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// The `Content-Range` field-value for a single-part `206` response,
    /// e.g. `bytes 2-5/10`.
    pub fn content_range_value(&self, total_len: u64) -> String {
        format!("bytes {}-{}/{total_len}", self.start, self.end)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RangeParseError;

/// Parses a `Range` header value against `total_len`, dropping ranges the
/// RFC calls unsatisfiable (a `first-byte-pos` at or beyond `total_len`, or a
/// zero-length suffix) rather than erroring on them -- only a header that
/// does not even look like a byte-range-set is [`RangeParseError`]. An empty
/// returned `Vec` means every range was individually unsatisfiable (the
/// caller's cue to answer `416`), which per RFC 7233 is different from a
/// syntactically invalid header (which this crate treats the same as "no
/// `Range` header" -- see `origin.rs`).
pub fn parse_range_header(value: &str, total_len: u64) -> Result<Vec<ByteRange>, RangeParseError> {
    let spec = value.trim().strip_prefix("bytes=").ok_or(RangeParseError)?;
    if spec.trim().is_empty() {
        return Err(RangeParseError);
    }
    let mut ranges = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err(RangeParseError);
        }
        let (start_s, end_s) = part.split_once('-').ok_or(RangeParseError)?;
        if start_s.is_empty() {
            // Suffix range "-N": the last N bytes.
            let suffix_len: u64 = end_s.parse().map_err(|_| RangeParseError)?;
            if suffix_len == 0 || total_len == 0 {
                continue; // unsatisfiable, not malformed
            }
            let start = total_len.saturating_sub(suffix_len);
            ranges.push(ByteRange {
                start,
                end: total_len - 1,
            });
        } else {
            let start: u64 = start_s.parse().map_err(|_| RangeParseError)?;
            if start >= total_len {
                continue; // unsatisfiable
            }
            let end = if end_s.is_empty() {
                total_len - 1
            } else {
                let requested_end: u64 = end_s.parse().map_err(|_| RangeParseError)?;
                if requested_end < start {
                    return Err(RangeParseError);
                }
                requested_end.min(total_len - 1)
            };
            ranges.push(ByteRange { start, end });
        }
    }
    Ok(ranges)
}

/// Builds one deterministic boundary token for a `multipart/byteranges`
/// response. Deterministic (a hash of the request shape, not random) so
/// `ebr-origin` needs no RNG dependency at all -- only `ebr-netem-proxy`'s
/// jitter/loss models need one (see `rng.rs`).
pub fn multipart_boundary(path: &str, ranges: &[ByteRange]) -> String {
    let seed = format!("{path}|{ranges:?}");
    format!(
        "EBR_NETEM_{}",
        &ebr_common::hash::sha256_hex(seed.as_bytes())[..24]
    )
}

/// One part's content type inside a `multipart/byteranges` body. This crate
/// always serves opaque archive bytes, never text, so every part uses the
/// same content type; kept as a named constant rather than repeated in
/// `origin.rs` and its tests.
pub const PART_CONTENT_TYPE: &str = "application/octet-stream";

/// Renders the exact bytes of a `multipart/byteranges` body (RFC 7233
/// §4.1): each part is `--boundary\r\n`, its headers, a blank line, the
/// range's bytes, then `\r\n`; the body ends with `--boundary--\r\n`.
pub fn build_multipart_body(
    boundary: &str,
    total_len: u64,
    file_bytes: &[u8],
    ranges: &[ByteRange],
) -> Vec<u8> {
    let mut out = Vec::new();
    for range in ranges {
        out.extend_from_slice(b"--");
        out.extend_from_slice(boundary.as_bytes());
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(format!("Content-Type: {PART_CONTENT_TYPE}\r\n").as_bytes());
        out.extend_from_slice(
            format!(
                "Content-Range: {}\r\n",
                range.content_range_value(total_len)
            )
            .as_bytes(),
        );
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(&file_bytes[range.start as usize..=range.end as usize]);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"--");
    out.extend_from_slice(boundary.as_bytes());
    out.extend_from_slice(b"--\r\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_simple_single_range() {
        let ranges = parse_range_header("bytes=2-5", 10).unwrap();
        assert_eq!(ranges, vec![ByteRange { start: 2, end: 5 }]);
        assert_eq!(ranges[0].content_range_value(10), "bytes 2-5/10");
    }

    #[test]
    fn open_ended_range_reaches_the_last_byte() {
        let ranges = parse_range_header("bytes=9500-", 10_000).unwrap();
        assert_eq!(
            ranges,
            vec![ByteRange {
                start: 9500,
                end: 9999
            }]
        );
    }

    #[test]
    fn suffix_range_is_the_last_n_bytes() {
        let ranges = parse_range_header("bytes=-500", 10_000).unwrap();
        assert_eq!(
            ranges,
            vec![ByteRange {
                start: 9500,
                end: 9999
            }]
        );
    }

    #[test]
    fn suffix_range_longer_than_the_file_is_clamped_to_the_whole_file() {
        let ranges = parse_range_header("bytes=-500", 100).unwrap();
        assert_eq!(ranges, vec![ByteRange { start: 0, end: 99 }]);
    }

    #[test]
    fn end_beyond_length_is_clamped_not_rejected() {
        let ranges = parse_range_header("bytes=0-999999", 10).unwrap();
        assert_eq!(ranges, vec![ByteRange { start: 0, end: 9 }]);
    }

    #[test]
    fn multiple_ranges_parse_in_order() {
        let ranges = parse_range_header("bytes=0-1,4-5,8-", 10).unwrap();
        assert_eq!(
            ranges,
            vec![
                ByteRange { start: 0, end: 1 },
                ByteRange { start: 4, end: 5 },
                ByteRange { start: 8, end: 9 },
            ]
        );
    }

    #[test]
    fn a_range_entirely_past_the_end_is_dropped_as_unsatisfiable_not_an_error() {
        let ranges = parse_range_header("bytes=10-20", 10).unwrap();
        assert!(ranges.is_empty());
    }

    #[test]
    fn zero_length_file_makes_every_range_unsatisfiable() {
        assert!(parse_range_header("bytes=0-0", 0).unwrap().is_empty());
        assert!(parse_range_header("bytes=-1", 0).unwrap().is_empty());
    }

    #[test]
    fn rejects_headers_missing_the_bytes_prefix() {
        assert_eq!(parse_range_header("items=0-1", 10), Err(RangeParseError));
    }

    #[test]
    fn rejects_an_inverted_range() {
        assert_eq!(parse_range_header("bytes=5-2", 10), Err(RangeParseError));
    }

    #[test]
    fn rejects_non_numeric_bounds() {
        assert_eq!(parse_range_header("bytes=a-b", 10), Err(RangeParseError));
    }

    #[test]
    fn multipart_boundary_is_deterministic_and_shape_sensitive() {
        let ranges = vec![ByteRange { start: 0, end: 1 }];
        let a = multipart_boundary("/x.eb", &ranges);
        let b = multipart_boundary("/x.eb", &ranges);
        let c = multipart_boundary("/y.eb", &ranges);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("EBR_NETEM_"));
    }

    #[test]
    fn multipart_body_has_the_expected_rfc7233_framing() {
        let file = b"0123456789";
        let ranges = vec![
            ByteRange { start: 0, end: 1 },
            ByteRange { start: 4, end: 5 },
        ];
        let body = build_multipart_body("BOUND", 10, file, &ranges);
        let text = String::from_utf8(body).unwrap();
        assert_eq!(
            text,
            "--BOUND\r\n\
             Content-Type: application/octet-stream\r\n\
             Content-Range: bytes 0-1/10\r\n\
             \r\n\
             01\r\n\
             --BOUND\r\n\
             Content-Type: application/octet-stream\r\n\
             Content-Range: bytes 4-5/10\r\n\
             \r\n\
             45\r\n\
             --BOUND--\r\n"
        );
    }
}
