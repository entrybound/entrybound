# HTTP range access v1

Status: implemented as a `RandomReadSource` for `INDEXED` archives.

## Client and protocol

The implementation pins `reqwest 0.13.4` with its blocking API and Rustls TLS
backend, default features disabled. It is MIT/Apache-2.0 compatible; no native
TLS library or manual HTTP parser is introduced.

Initialization accepts only `http` and `https`, follows at most three redirects,
requests identity content encoding, and requires a successful metadata response
with one exact `Content-Length`, `Accept-Ranges: bytes`, a strong quoted ETag
(weak `W/` validators are refused), and no non-identity `Content-Encoding`.

Each range request carries `Range: bytes=start-end`, `If-Match` with the initial
ETag, and `Accept-Encoding: identity`. It must receive status 206, the same
strong ETag, an exact Content-Length, a literal matching
`Content-Range: bytes start-end/total`, and exactly the requested body length.
Status 200 is never a whole-body fallback. A 412, changed validator/length, or
final revalidation mismatch is source instability; malformed range framing is
corruption and unsupported server behavior is a typed refusal.

Passwords and identities remain client-side and are never sent to the server.

## Cache and coalescing

The session cache is revision-scoped with a caller-owned byte bound and
deterministic FIFO eviction. A contained read may be served from cache. Nearby
ranges having the same diagnostic purpose may be coalesced when gap, range, and
aggregate policies permit. Coalescing affects only request shape; verification
and logical output are identical when it is disabled.

## Security and leakage

TLS protects transport only for HTTPS; Entrybound hashes/AEAD remain mandatory.
A strong ETag pins server-declared revision while archive integrity mechanisms
authenticate fetched bytes. Random access never proves unread bytes.

The server observes URL, total object size, byte ranges, timing, and request
count. Crypto-v1 hides names and private metadata but does not hide physical
access patterns. This is not ORAM/PIR. S3 credentials, WebDAV, FTP, cloud APIs,
remote writes, and STREAM fallback are outside v1.

