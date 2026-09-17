//! JSON-lines result writer.
//!
//! Schema `ebr.harness.raw.v1`: one compact JSON object per line, each
//! carrying `schema`, `experiment_id`, `run_id`, `env_id`, an
//! auto-incrementing `seq`, a wall-clock `timestamp_utc`, an optional
//! `item_id`, and a free-form `payload` object the caller controls.
//!
//! This is a first cut for the Rust harness, not byte-identical with the
//! Python baselines' `ebr.raw.v1` schema already in
//! `research/raw/*/*.jsonl`. That schema additionally hash-chains rows
//! (`row_sha256`/`prev_row_sha256` over each row's own bytes) and carries
//! per-external-tool measurement detail (`command`, `checks`,
//! `item_fingerprint`, ...) that does not apply to an in-process Rust
//! measurement the same way. Reconciling the two -- and deciding whether the
//! harness should also hash-chain -- is future work for whichever agent
//! designs the C-exec experiment specs; this module documents the gap
//! instead of guessing at a reconciliation.

use crate::runenv::RunContext;
use serde::Serialize;
use serde_json::Value;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::Path;

pub const SCHEMA: &str = "ebr.harness.raw.v1";

#[derive(Debug, Serialize)]
pub struct ResultRecord<'a> {
    pub schema: &'a str,
    pub experiment_id: &'a str,
    pub run_id: &'a str,
    pub env_id: &'a str,
    pub seq: u64,
    pub timestamp_utc: String,
    pub item_id: Option<&'a str>,
    pub payload: Value,
}

/// Appends [`ResultRecord`]s to a sink, one per line, flushing after each so
/// a crash mid-experiment loses at most the in-flight record.
pub struct ResultWriter<W: Write> {
    sink: W,
    context: RunContext,
    seq: u64,
}

impl ResultWriter<BufWriter<File>> {
    /// Opens (creating if needed) a JSONL file at `path` in append mode --
    /// matching `research/raw/<experiment_id>/<run_id>.jsonl` conventions.
    pub fn create(path: &Path, context: RunContext) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(ResultWriter {
            sink: BufWriter::new(file),
            context,
            seq: 0,
        })
    }
}

impl<W: Write> ResultWriter<W> {
    pub fn new(sink: W, context: RunContext) -> Self {
        ResultWriter {
            sink,
            context,
            seq: 0,
        }
    }

    pub fn context(&self) -> &RunContext {
        &self.context
    }

    /// Writes one record. `payload` is caller-defined and un-schema'd beyond
    /// being a JSON value; put whatever the experiment measured in it.
    pub fn append(&mut self, item_id: Option<&str>, payload: Value) -> io::Result<()> {
        let record = ResultRecord {
            schema: SCHEMA,
            experiment_id: &self.context.experiment_id,
            run_id: &self.context.run_id,
            env_id: &self.context.env_id,
            seq: self.seq,
            timestamp_utc: crate::timing::now_utc_rfc3339(),
            item_id,
            payload,
        };
        let line = serde_json::to_string(&record).map_err(io::Error::other)?;
        self.sink.write_all(line.as_bytes())?;
        self.sink.write_all(b"\n")?;
        self.sink.flush()?;
        self.seq += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn writes_one_json_object_per_line_with_incrementing_seq() {
        let mut buf = Vec::new();
        let ctx = RunContext::with_run_id("EXP-SMOKE-000", "20260101T000000Z-aaaaaa", "test-env");
        {
            let mut writer = ResultWriter::new(&mut buf, ctx);
            writer.append(Some("item-a"), json!({"ok": true})).unwrap();
            writer
                .append(None, json!({"ok": false, "reason": "no item"}))
                .unwrap();
        }
        let text = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);

        let first: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first["schema"], SCHEMA);
        assert_eq!(first["experiment_id"], "EXP-SMOKE-000");
        assert_eq!(first["run_id"], "20260101T000000Z-aaaaaa");
        assert_eq!(first["env_id"], "test-env");
        assert_eq!(first["seq"], 0);
        assert_eq!(first["item_id"], "item-a");
        assert_eq!(first["payload"]["ok"], true);

        let second: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(second["seq"], 1);
        assert!(second["item_id"].is_null());
    }

    #[test]
    fn create_makes_parent_directories_and_appends_across_writers() {
        let dir =
            std::env::temp_dir().join(format!("ebr-common-results-test-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let path = dir.join("nested").join("run.jsonl");

        {
            let ctx = RunContext::with_run_id("EXP-A", "run-1", "env-1");
            let mut writer = ResultWriter::create(&path, ctx).unwrap();
            writer.append(Some("x"), json!({"n": 1})).unwrap();
        }
        {
            let ctx = RunContext::with_run_id("EXP-A", "run-1", "env-1");
            let mut writer = ResultWriter::create(&path, ctx).unwrap();
            writer.append(Some("y"), json!({"n": 2})).unwrap();
        }

        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 2);

        std::fs::remove_dir_all(&dir).ok();
    }
}
