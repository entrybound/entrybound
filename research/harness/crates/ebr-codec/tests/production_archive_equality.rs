//! Verifies that `entrybound::research::codec::encode_payload`, given the
//! exact `TransformPlan` and plaintext production's own real packer
//! (`entrybound::archive::filesystem::pack_directory`, not a research-only
//! path) assigned to a Chunk, reproduces the exact stored payload bytes
//! physically present in the packed archive -- across all four
//! `CompressionProfile` values, using the INDEXED layout's own `Index`
//! (`ChunkLocation { offset, stored_len }` in `EncodedArchive.bytes`) to
//! find each chunk's stored bytes. `offset` is the Chunk frame's header
//! start (not its payload start) and `stored_len` is the payload-only
//! length after that fixed-size header; `chunk_frame_header_len` (exposed
//! read-only under `research-internals`, applied here exactly as
//! `entrybound::ecf::container` applies it when building this same Index)
//! gives that header length. This is the "production-plan byte equality
//! with archive payloads" check the crate's task calls for: it does not
//! exercise a reimplementation of anything, only that this crate's wrapper
//! functions in `src/production.rs` call the exact same production
//! functions the same way the real planner and packer do.
//!
//! Only `PlanMode::Independent` chunks are checked (plain `encode_payload`
//! is what applies there); dictionary/prefix-mode chunks need their
//! dictionary/prefix bytes too, which `production.rs`'s own unit tests
//! already cover directly. A JPEG whole-object region member (`stored_len ==
//! 0`, its bytes owned by the region's first chunk) is skipped for the same
//! reason `research-internals.md` gives: `Index` does not record it as an
//! independently stored payload.

use entrybound::archive::{PackOptions, pack_directory};
use entrybound::ecf::{FEATURE_CROSS_FILE_COMPRESSION_V1, FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1};
use entrybound::planner::CompressionProfile;
use entrybound::research::{codec, ecf as research_ecf};

fn make_fixture_tree(root: &std::path::Path) {
    std::fs::create_dir_all(root).unwrap();

    // Highly repetitive content: the planner should favor a real compressor.
    let text: Vec<u8> = b"entrybound research harness archive equality fixture; "
        .iter()
        .cycle()
        .take(400_000)
        .copied()
        .collect();
    std::fs::write(root.join("repetitive.txt"), &text).unwrap();

    // Higher-entropy content: a different candidate may win, or STORE.
    let mut state = 0x243f_6a88_85a3_08d3_u64;
    let pseudo_random: Vec<u8> = (0..200_000)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state & 0xff) as u8
        })
        .collect();
    std::fs::write(root.join("pseudo-random.bin"), &pseudo_random).unwrap();

    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::write(root.join("sub").join("small.txt"), b"a small file\n").unwrap();
}

#[test]
fn production_plan_reencode_matches_the_packed_archives_stored_bytes() {
    for profile in [
        CompressionProfile::Fast,
        CompressionProfile::Balanced,
        CompressionProfile::Dense,
        CompressionProfile::Extreme,
    ] {
        let dir = std::env::temp_dir().join(format!(
            "ebr-codec-archive-equality-{profile:?}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        make_fixture_tree(&dir);

        let options = PackOptions {
            profile,
            ..PackOptions::default()
        };
        let encoded = pack_directory(&dir, options).unwrap();
        assert!(
            encoded.archive.index.present,
            "expected the default INDEXED layout to carry an Index"
        );

        // `ChunkLocation::offset` (see `entrybound::eam::Index`) is the
        // absolute offset of the Chunk frame's *header* start, not its
        // stored payload -- confirmed empirically against this archive's
        // own bytes below, and matching how `entrybound::ecf::container`
        // builds the Index during encoding (`offset` is the frame start,
        // `stored_len` the payload-only length after the header). The
        // header length is constant for the whole archive: it depends only
        // on two archive-wide feature bits, not on any one chunk.
        let features = encoded.archive.descriptor.features.incompat;
        let extended = features & FEATURE_CROSS_FILE_COMPRESSION_V1 != 0;
        let whole_object = features & FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1 != 0;
        let header_len = usize::try_from(research_ecf::chunk_frame_header_len(extended)).unwrap();
        let _ = whole_object; // only used to justify the comment above

        let mut independent_chunks_checked = 0usize;
        for (chunk_id, chunk) in &encoded.archive.content_store.chunks {
            // `Chunk::plan_ref` is a `TransformPlan::plan_id` to look up, not
            // a positional index into `archive.transform_plans` (confirmed
            // against `entrybound::ecf::container`'s own `plans.get(&chunk.plan_ref)`
            // lookup, keyed by `plan_id`).
            let plan = encoded
                .archive
                .transform_plans
                .iter()
                .find(|plan| plan.plan_id == chunk.plan_ref)
                .expect("every Chunk::plan_ref must resolve to a recorded TransformPlan");
            if codec::plan_mode(plan).unwrap() != codec::PlanMode::Independent {
                // Dictionary/prefix modes need side bytes this fixture-only
                // test does not build; see production.rs's own tests.
                continue;
            }
            let Some(location) = encoded.archive.index.chunks.get(chunk_id) else {
                continue;
            };
            if location.stored_len == 0 {
                // A whole-object JPEG region member: its bytes are owned by
                // the region's first chunk, not stored at this location.
                continue;
            }

            let reencoded = codec::encode_payload(plan, &chunk.plaintext).unwrap();
            let start = usize::try_from(location.offset).unwrap() + header_len;
            let end = start + usize::try_from(location.stored_len).unwrap();
            assert_eq!(
                reencoded,
                &encoded.bytes[start..end],
                "chunk {chunk_id:?} under profile {profile:?} did not re-encode to the \
                 archive's stored bytes"
            );
            independent_chunks_checked += 1;
        }
        assert!(
            independent_chunks_checked > 0,
            "expected at least one independent-mode chunk under profile {profile:?}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
