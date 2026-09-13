//! Prints gear-norm-v1 chunk lengths (run-length encoded) for every file argument and every v2 policy,
//! using the real `entrybound::chunker::chunk_ranges`.  One JSON object per line:
//! {"file": "...", "policy": "...", "chunks": N, "rle": [[len, count], ...]}

use entrybound::chunker::{BALANCED_V2, DENSE_V2, EXTREME_V2, FAST_V2, chunk_ranges};

fn main() {
    let policies = [("fast-v2", FAST_V2), ("balanced-v2", BALANCED_V2), ("dense-v2", DENSE_V2), ("extreme-v2", EXTREME_V2)];
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        for (name, params) in policies {
            let ranges = chunk_ranges(&data, params).unwrap_or_else(|e| panic!("chunk {path}: {e:?}"));
            let mut rle: Vec<(usize, usize)> = Vec::new();
            for r in ranges.iter() {
                match rle.last_mut() {
                    Some(last) if last.0 == r.len() => last.1 += 1,
                    _ => rle.push((r.len(), 1)),
                }
            }
            let body: Vec<String> = rle.iter().map(|(l, c)| format!("[{l},{c}]")).collect();
            println!(
                "{{\"file\":{:?},\"policy\":\"{}\",\"chunks\":{},\"rle\":[{}]}}",
                path,
                name,
                ranges.len(),
                body.join(",")
            );
        }
    }
}
