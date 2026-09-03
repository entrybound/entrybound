//! Independent bounded tar-family observation and strict reconciliation.
//!
//! POSIX, pax, and GNU claims remain foreign evidence until this module's
//! strict resolver projects regular files/directories into EAM.

use std::collections::{BTreeMap, BTreeSet};

use super::lom::{
    ConflictClass, LegacyArchiveObservation, LegacyAuthority, LegacyConflict,
    LegacyEntryObservation, LegacyEvidenceLocation, LegacyFieldObservation, LegacyObservedValue,
    LegacyResolution, ObservationValidity,
};
use super::stream::DecodedTransport;
use crate::archive::plan_observed_archive;
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ContentRef, ConversionProvenance, ConversionResolution, Entry, EntryData,
    EntryIdentity, FidelityIssue, FidelityReport, LinkTarget, LogicalPath, MetadataItem,
    MetadataSet, Timestamp, TimestampPrecision,
};
use crate::identity::{hardlink_group_id, sha256_exact};
use crate::planner::CompressionProfile;

const BLOCK: usize = 512;

/// Caller-owned tar parsing and materialization limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TarImportPolicy {
    pub max_archive_bytes: u64,
    pub max_entries: u64,
    pub max_entry_bytes: u64,
    pub max_total_file_bytes: u64,
    pub max_name_bytes: u64,
    pub max_extension_bytes: u64,
    pub max_pax_records: u64,
    pub max_observations: u64,
    pub max_observations_per_subject: u64,
    pub max_conflicts: u64,
    pub max_resolutions: u64,
}

impl Default for TarImportPolicy {
    fn default() -> Self {
        Self {
            max_archive_bytes: 16 * 1024 * 1024 * 1024,
            max_entries: 1_000_000,
            max_entry_bytes: 4 * 1024 * 1024 * 1024,
            max_total_file_bytes: 16 * 1024 * 1024 * 1024,
            max_name_bytes: 1024 * 1024,
            max_extension_bytes: 64 * 1024 * 1024,
            max_pax_records: 1_000_000,
            max_observations: 8_000_000,
            max_observations_per_subject: 4096,
            max_conflicts: 1_000_000,
            max_resolutions: 1_000_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TarConversionReport {
    pub observation: LegacyArchiveObservation,
    pub resolutions: Box<[ConversionResolution]>,
    pub synthesized_ancestors: Box<[LogicalPath]>,
    pub layers: Box<[String]>,
    pub wrapper_members: u64,
    pub decoded_child_digest: Option<crate::eam::Digest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TarImportResult {
    pub archive: Archive,
    pub report: TarConversionReport,
}

/// Immutable tar evidence and the decoded child bytes needed for resolution.
#[derive(Clone, Debug)]
pub struct TarObservation {
    lom: LegacyArchiveObservation,
    source: Box<[u8]>,
    entries: Box<[ObservedTarEntry]>,
    unsupported_metadata: BTreeSet<String>,
    layers: Vec<String>,
    wrapper_members: u64,
    decoded_child_digest: Option<crate::eam::Digest>,
}

impl TarObservation {
    #[must_use]
    pub const fn lom(&self) -> &LegacyArchiveObservation {
        &self.lom
    }

    /// Adds independently observed transport evidence without changing tar
    /// source locations, which remain relative to the decoded-child space.
    pub fn attach_transport(&mut self, transport: &DecodedTransport) {
        let mut fields = transport.observation.archive_fields.to_vec();
        for field in &mut self.lom.archive_fields {
            if field.semantic_field == "layer.ordinal" && field.authority.structure == "tar-layer" {
                field.raw_value = 1_u64.to_be_bytes().to_vec().into_boxed_slice();
                field.interpreted_value = Some(LegacyObservedValue::Unsigned(1));
            }
        }
        fields.extend(self.lom.archive_fields.iter().cloned());
        self.lom.archive_fields = fields.into_boxed_slice();
        self.lom.source_format = format!("tar+{}", transport.format.as_str());
        self.lom.source_digest = transport.observation.source_digest;
        self.layers.insert(0, transport.format.as_str().to_owned());
        self.wrapper_members = transport.member_count;
        self.decoded_child_digest = Some(transport.decoded_digest);
    }
}

#[derive(Clone, Debug)]
struct PaxClaim {
    value: String,
    authority: LegacyAuthority,
    location: LegacyEvidenceLocation,
}

type ParsedPax = (
    BTreeMap<String, PaxClaim>,
    Vec<LegacyFieldObservation<LegacyObservedValue>>,
);

struct HeaderClaims<'a> {
    path: &'a [u8],
    size: u64,
    mtime: i64,
    uid: u64,
    gid: u64,
    authority: &'a LegacyAuthority,
    location: LegacyEvidenceLocation,
}

#[derive(Clone, Debug)]
struct ObservedTarEntry {
    payload_offset: usize,
    payload_size: u64,
    header_name: Box<[u8]>,
    header_linkname: Box<[u8]>,
    header_mode: u64,
    header_uid: u64,
    header_gid: u64,
    header_mtime: i64,
    typeflag: u8,
    global_pax: BTreeMap<String, PaxClaim>,
    local_pax: BTreeMap<String, PaxClaim>,
    gnu_long_name: Option<PaxClaim>,
    gnu_long_link: Option<PaxClaim>,
}

#[derive(Clone, Debug)]
enum ResolvedKind {
    Directory,
    File,
    Symlink(LinkTarget),
    Hardlink(LogicalPath),
}

#[derive(Clone, Debug)]
struct ResolvedEntry {
    path: LogicalPath,
    components: Vec<String>,
    kind: ResolvedKind,
    executable: bool,
    mode: u32,
    uid: u32,
    gid: u32,
    mtime: Timestamp,
    plaintext: Box<[u8]>,
}

/// Fast structural preflight used only for dispatch; full acceptance always
/// runs [`observe`].
#[must_use]
pub fn looks_like_tar(source: &[u8]) -> bool {
    if source.len() < BLOCK * 2 || !source.len().is_multiple_of(BLOCK) {
        return false;
    }
    if source[..BLOCK].iter().all(|byte| *byte == 0) {
        return source[..BLOCK * 2].iter().all(|byte| *byte == 0);
    }
    checksum_valid(&source[..BLOCK])
}

pub fn observe(source: &[u8], policy: TarImportPolicy) -> Result<TarObservation> {
    policy_check(
        u64::try_from(source.len()).unwrap_or(u64::MAX) <= policy.max_archive_bytes,
        "tar source exceeds max_archive_bytes",
    )?;
    if source.len() < BLOCK * 2 || !source.len().is_multiple_of(BLOCK) {
        return Err(structure(
            "tar source is not a complete 512-byte record sequence",
        ));
    }
    let source_digest = sha256_exact(source);
    let mut cursor = 0_usize;
    let mut actual_ordinal = 0_u64;
    let mut physical_ordinal = 0_u64;
    let mut archive_fields = vec![
        field_u64("layer.ordinal", &authority("tar-layer", 0), 0, 0, 0),
        field_text(
            "layer.format",
            &authority("tar-layer", 0),
            "tar",
            0,
            BLOCK.min(source.len()),
        ),
    ];
    let mut observation_count = u64::try_from(archive_fields.len()).unwrap_or(u64::MAX);
    let mut lom_entries = Vec::new();
    let mut entries = Vec::new();
    let mut conflicts = Vec::new();
    let mut unsupported_metadata = BTreeSet::new();
    let mut global_pax = BTreeMap::<String, PaxClaim>::new();
    let mut local_pax = BTreeMap::<String, PaxClaim>::new();
    let mut pending_fields = Vec::new();
    let mut gnu_long_name: Option<PaxClaim> = None;
    let mut gnu_long_link: Option<PaxClaim> = None;
    let mut pax_records = 0_u64;
    let mut found_end = false;

    while cursor < source.len() {
        let header = slice(source, cursor, BLOCK, "tar header")?;
        if header.iter().all(|byte| *byte == 0) {
            let second = slice(source, cursor + BLOCK, BLOCK, "second tar end block")?;
            if !second.iter().all(|byte| *byte == 0) {
                return Err(structure("tar has only one zero end block"));
            }
            let trailing = &source[cursor + BLOCK * 2..];
            if trailing.iter().any(|byte| *byte != 0) {
                return Err(structure("tar has nonzero bytes after its end marker"));
            }
            archive_fields.push(field_u64(
                "archive.trailing_zero_blocks",
                &authority("tar-end", 0),
                u64::try_from(trailing.len() / BLOCK + 2).unwrap_or(u64::MAX),
                cursor,
                source.len() - cursor,
            ));
            observation_count = observation_count.saturating_add(1);
            policy_check(
                observation_count <= policy.max_observations,
                "tar observations exceed policy",
            )?;
            found_end = true;
            break;
        }
        if !checksum_valid(header) {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::TarChecksumMismatch,
                format!("tar header checksum mismatch at byte {cursor}"),
            ));
        }
        let header_authority = authority("tar-header", physical_ordinal);
        physical_ordinal = physical_ordinal.saturating_add(1);
        let name = string_field(header, 0, 100);
        let mode = unsigned_number(&header[100..108], "tar mode")?;
        let uid = unsigned_number(&header[108..116], "tar uid")?;
        let gid = unsigned_number(&header[116..124], "tar gid")?;
        let size = unsigned_number(&header[124..136], "tar size")?;
        let mtime = signed_number(&header[136..148], "tar mtime")?;
        let typeflag = header[156];
        let linkname = string_field(header, 157, 100);
        let magic = string_field(header, 257, 6);
        let version = string_field(header, 263, 2);
        let uname = string_field(header, 265, 32);
        let gname = string_field(header, 297, 32);
        let devmajor = unsigned_number(&header[329..337], "tar devmajor")?;
        let devminor = unsigned_number(&header[337..345], "tar devminor")?;
        let prefix = string_field(header, 345, 155);
        let header_name = joined_header_name(&prefix, &name, policy.max_name_bytes)?;
        let is_extension = matches!(typeflag, b'g' | b'x' | b'L' | b'K');
        let extent_size = if is_extension {
            size
        } else {
            local_pax
                .get("size")
                .or_else(|| global_pax.get("size"))
                .map_or(Ok(size), |claim| {
                    claim
                        .value
                        .parse::<u64>()
                        .map_err(|_| structure("pax size is not a canonical unsigned integer"))
                })?
        };
        policy_check(
            extent_size <= policy.max_entry_bytes,
            "tar entry size exceeds policy",
        )?;
        let payload_offset = cursor
            .checked_add(BLOCK)
            .ok_or_else(|| structure("tar payload offset overflow"))?;
        let payload_size = usize::try_from(extent_size)
            .map_err(|_| structure("tar entry size is not addressable"))?;
        let padded_size = round_block(payload_size)?;
        slice(
            source,
            payload_offset,
            padded_size,
            "tar payload and padding",
        )?;
        if source[payload_offset + payload_size..payload_offset + padded_size]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(structure("tar file padding contains nonzero bytes"));
        }

        let mut fields = vec![
            field_bytes("header.block", &header_authority, header, cursor),
            field_bytes("path.name", &header_authority, &name, cursor),
            field_u64("mode", &header_authority, mode, cursor + 100, 8),
            field_u64("uid", &header_authority, uid, cursor + 108, 8),
            field_u64("gid", &header_authority, gid, cursor + 116, 8),
            field_u64("size", &header_authority, size, cursor + 124, 12),
            field_i64("mtime", &header_authority, mtime, cursor + 136, 12),
            field_bytes(
                "header.checksum",
                &header_authority,
                &header[148..156],
                cursor + 148,
            ),
            field_u64(
                "typeflag",
                &header_authority,
                typeflag.into(),
                cursor + 156,
                1,
            ),
            field_bytes("linkpath", &header_authority, &linkname, cursor + 157),
            field_bytes("format.magic", &header_authority, &magic, cursor + 257),
            field_bytes("format.version", &header_authority, &version, cursor + 263),
            field_bytes("uname", &header_authority, &uname, cursor + 265),
            field_bytes("gname", &header_authority, &gname, cursor + 297),
            field_u64("devmajor", &header_authority, devmajor, cursor + 329, 8),
            field_u64("devminor", &header_authority, devminor, cursor + 337, 8),
            field_bytes("path.prefix", &header_authority, &prefix, cursor + 345),
            field_u64(
                "payload.extent",
                &header_authority,
                extent_size,
                payload_offset,
                payload_size,
            ),
            field_u64(
                "payload.padding",
                &header_authority,
                u64::try_from(padded_size - payload_size).unwrap_or(u64::MAX),
                payload_offset + payload_size,
                padded_size - payload_size,
            ),
        ];
        observation_count =
            observation_count.saturating_add(u64::try_from(fields.len()).unwrap_or(u64::MAX));
        unsupported_metadata.extend(["tar.uname".to_owned(), "tar.gname".to_owned()]);
        let payload = &source[payload_offset..payload_offset + payload_size];
        match typeflag {
            b'g' | b'x' => {
                policy_check(
                    size <= policy.max_extension_bytes,
                    "pax data exceeds policy",
                )?;
                let parsed = parse_pax(
                    payload,
                    payload_offset,
                    &authority(
                        if typeflag == b'g' {
                            "pax-global"
                        } else {
                            "pax-local"
                        },
                        physical_ordinal - 1,
                    ),
                    &mut conflicts,
                    &mut pax_records,
                    policy,
                )?;
                for key in parsed.0.keys().filter(|key| !known_pax_key(key)) {
                    unsupported_metadata.insert(format!("tar.pax.{key}"));
                }
                if let Some(mtime) = parsed.0.get("mtime") {
                    let fractional_digits = mtime
                        .value
                        .split_once('.')
                        .map_or(0, |(_, fractional)| fractional.len());
                    if !matches!(fractional_digits, 0 | 2 | 6 | 7 | 9) {
                        unsupported_metadata
                            .insert(format!("tar.mtime-precision-{fractional_digits}-digits"));
                    }
                }
                observation_count = observation_count
                    .saturating_add(u64::try_from(parsed.1.len()).unwrap_or(u64::MAX));
                fields.extend(parsed.1);
                pending_fields.extend(fields);
                if typeflag == b'g' {
                    merge_pax(&mut global_pax, parsed.0, true, &mut conflicts)?;
                } else {
                    merge_pax(&mut local_pax, parsed.0, false, &mut conflicts)?;
                }
            }
            b'L' | b'K' => {
                policy_check(
                    size <= policy.max_extension_bytes,
                    "GNU long value exceeds policy",
                )?;
                let value = trim_nul(payload);
                let text = std::str::from_utf8(value)
                    .map_err(|_| unsafe_path("GNU long name/link is not UTF-8"))?
                    .to_owned();
                policy_check(
                    u64::try_from(value.len()).unwrap_or(u64::MAX) <= policy.max_name_bytes,
                    "GNU long name/link exceeds policy",
                )?;
                let claim = PaxClaim {
                    value: text,
                    authority: authority(
                        if typeflag == b'L' {
                            "gnu-long-name"
                        } else {
                            "gnu-long-link"
                        },
                        physical_ordinal - 1,
                    ),
                    location: location(payload_offset, payload_size),
                };
                let slot = if typeflag == b'L' {
                    &mut gnu_long_name
                } else {
                    &mut gnu_long_link
                };
                if let Some(previous) = slot.as_ref()
                    && previous.value != claim.value
                {
                    conflicts.push(conflict(
                        if typeflag == b'L' { "path" } else { "linkpath" },
                        previous,
                        &claim,
                        ConflictClass::Divergence,
                        None,
                    ));
                }
                *slot = Some(claim.clone());
                fields.push(field_text(
                    if typeflag == b'L' {
                        "path.gnu-long"
                    } else {
                        "linkpath.gnu-long"
                    },
                    &claim.authority,
                    &claim.value,
                    payload_offset,
                    payload_size,
                ));
                observation_count = observation_count.saturating_add(1);
                pending_fields.extend(fields);
            }
            _ => {
                fields.append(&mut pending_fields);
                let repeated_field_start = fields.len();
                fields.extend(pax_fields(&global_pax));
                fields.extend(pax_fields(&local_pax));
                if let Some(claim) = &gnu_long_name {
                    fields.push(field_text(
                        "path.gnu-long",
                        &claim.authority,
                        &claim.value,
                        usize::try_from(claim.location.offset).unwrap_or(0),
                        usize::try_from(claim.location.length).unwrap_or(0),
                    ));
                }
                if let Some(claim) = &gnu_long_link {
                    fields.push(field_text(
                        "linkpath.gnu-long",
                        &claim.authority,
                        &claim.value,
                        usize::try_from(claim.location.offset).unwrap_or(0),
                        usize::try_from(claim.location.length).unwrap_or(0),
                    ));
                }
                observation_count = observation_count.saturating_add(
                    u64::try_from(fields.len() - repeated_field_start).unwrap_or(u64::MAX),
                );
                classify_overrides(
                    HeaderClaims {
                        path: &header_name,
                        size,
                        mtime,
                        uid,
                        gid,
                        authority: &header_authority,
                        location: location(cursor, BLOCK),
                    },
                    &global_pax,
                    &local_pax,
                    gnu_long_name.as_ref(),
                    &mut conflicts,
                );
                policy_check(
                    u64::try_from(fields.len()).unwrap_or(u64::MAX)
                        <= policy.max_observations_per_subject,
                    "tar observations for one entry exceed policy",
                )?;
                lom_entries.push(LegacyEntryObservation {
                    ordinal: actual_ordinal,
                    fields: fields.into_boxed_slice(),
                });
                entries.push(ObservedTarEntry {
                    payload_offset,
                    payload_size: extent_size,
                    header_name,
                    header_linkname: linkname,
                    header_mode: mode,
                    header_uid: uid,
                    header_gid: gid,
                    header_mtime: mtime,
                    typeflag,
                    global_pax: global_pax.clone(),
                    local_pax: std::mem::take(&mut local_pax),
                    gnu_long_name: gnu_long_name.take(),
                    gnu_long_link: gnu_long_link.take(),
                });
                actual_ordinal = actual_ordinal.saturating_add(1);
                policy_check(
                    actual_ordinal <= policy.max_entries,
                    "tar entry count exceeds policy",
                )?;
            }
        }
        policy_check(
            observation_count <= policy.max_observations,
            "tar observations exceed policy",
        )?;
        policy_check(
            u64::try_from(conflicts.len()).unwrap_or(u64::MAX) <= policy.max_conflicts,
            "tar conflicts exceed policy",
        )?;
        cursor = payload_offset
            .checked_add(padded_size)
            .ok_or_else(|| structure("tar next-header offset overflow"))?;
    }
    if !found_end {
        return Err(structure("tar end marker is missing"));
    }
    if !local_pax.is_empty() || gnu_long_name.is_some() || gnu_long_link.is_some() {
        return Err(structure(
            "tar ends with an extension record that has no target entry",
        ));
    }
    policy_check(
        observation_count <= policy.max_observations,
        "tar observations exceed policy",
    )?;
    policy_check(
        u64::try_from(conflicts.len()).unwrap_or(u64::MAX) <= policy.max_conflicts,
        "tar conflicts exceed policy",
    )?;
    Ok(TarObservation {
        lom: LegacyArchiveObservation {
            source_format: "tar".to_owned(),
            source_digest,
            archive_fields: archive_fields.into_boxed_slice(),
            entries: lom_entries.into_boxed_slice(),
            conflicts: conflicts.into_boxed_slice(),
        },
        source: source.to_vec().into_boxed_slice(),
        entries: entries.into_boxed_slice(),
        unsupported_metadata,
        layers: vec!["tar".to_owned()],
        wrapper_members: 0,
        decoded_child_digest: None,
    })
}

pub fn resolve_strict(
    observation: TarObservation,
    policy: TarImportPolicy,
    profile: CompressionProfile,
) -> Result<TarImportResult> {
    refuse_unresolved(&observation.lom.conflicts)?;
    let mut resolutions = observation
        .lom
        .conflicts
        .iter()
        .filter_map(conflict_resolution)
        .collect::<Vec<_>>();
    policy_check(
        u64::try_from(resolutions.len()).unwrap_or(u64::MAX) <= policy.max_resolutions,
        "tar resolutions exceed policy",
    )?;
    if let Some(decoded_digest) = observation.decoded_child_digest {
        resolutions.push(ConversionResolution {
            conflict_class: ConflictClass::Refinement.as_str().to_owned(),
            semantic_field: "layer.transport-decoded-child".to_owned(),
            authorities: Box::from([
                observation.layers[0].clone(),
                "tar decoded-child bytes".to_owned(),
            ]),
            observed_values: Box::from([
                observation.lom.source_digest.to_string(),
                decoded_digest.to_string(),
            ]),
            action: format!(
                "verified {} transport ({} members) before strict tar observation",
                observation.layers[0], observation.wrapper_members
            ),
        });
        policy_check(
            u64::try_from(resolutions.len()).unwrap_or(u64::MAX) <= policy.max_resolutions,
            "tar resolutions exceed policy",
        )?;
    }
    let mut resolved = Vec::new();
    let mut total_bytes = 0_u64;
    for entry in &observation.entries {
        let item = resolve_entry(entry, &observation.source, &mut resolutions)?;
        policy_check(
            u64::try_from(resolutions.len()).unwrap_or(u64::MAX) <= policy.max_resolutions,
            "tar resolutions exceed policy",
        )?;
        if matches!(item.kind, ResolvedKind::File) {
            total_bytes = total_bytes
                .checked_add(u64::try_from(item.plaintext.len()).unwrap_or(u64::MAX))
                .ok_or_else(|| policy_error("tar total file bytes overflow"))?;
            policy_check(
                total_bytes <= policy.max_total_file_bytes,
                "tar output exceeds policy",
            )?;
        }
        resolved.push(item);
    }
    let mut by_path = BTreeMap::<LogicalPath, ResolvedEntry>::new();
    for entry in resolved {
        if by_path.insert(entry.path.clone(), entry).is_some() {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateLogicalPath,
                "duplicate reconciled tar path",
            ));
        }
    }
    let mut kinds = by_path
        .iter()
        .map(|(path, entry)| (path.clone(), matches!(entry.kind, ResolvedKind::Directory)))
        .collect::<BTreeMap<_, _>>();
    let mut synthesized = BTreeSet::new();
    for entry in by_path.values() {
        for depth in 1..entry.components.len() {
            let ancestor = LogicalPath::from_utf8(&entry.components[..depth])?;
            match kinds.get(&ancestor) {
                Some(true) => {}
                Some(false) => {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::FileAsAncestor,
                        format!("tar file {ancestor} is an ancestor"),
                    ));
                }
                None => {
                    kinds.insert(ancestor.clone(), true);
                    synthesized.insert(ancestor.clone());
                    resolutions.push(ConversionResolution {
                        conflict_class: ConflictClass::Omission.as_str().to_owned(),
                        semantic_field: format!("directory:{ancestor}"),
                        authorities: Box::from(["tar child paths".to_owned()]),
                        observed_values: Box::from(["directory entry omitted".to_owned()]),
                        action: "synthesized explicit ancestor required by EAM".to_owned(),
                    });
                    policy_check(
                        u64::try_from(resolutions.len()).unwrap_or(u64::MAX)
                            <= policy.max_resolutions,
                        "tar resolutions exceed policy",
                    )?;
                }
            }
        }
    }
    let mut entries = synthesized
        .iter()
        .cloned()
        .map(|path| {
            Entry::new(
                path,
                EntryData::Directory,
                MetadataSet::default(),
                EntryIdentity::default(),
            )
        })
        .collect::<Vec<_>>();
    let mut hardlink_sets = BTreeMap::<LogicalPath, Vec<LogicalPath>>::new();
    for item in by_path.values() {
        if let ResolvedKind::Hardlink(target) = &item.kind {
            let target_entry = by_path.get(target).ok_or_else(|| {
                irreconcilable(format!(
                    "tar hardlink {} references missing target {target}",
                    item.path
                ))
            })?;
            if !matches!(target_entry.kind, ResolvedKind::File) {
                return Err(irreconcilable(format!(
                    "tar hardlink {} does not reference a regular file",
                    item.path
                )));
            }
            hardlink_sets
                .entry(target.clone())
                .or_insert_with(|| vec![target.clone()])
                .push(item.path.clone());
        }
    }
    let mut group_by_path = BTreeMap::<LogicalPath, crate::eam::Digest>::new();
    for (target, mut paths) in hardlink_sets {
        paths.sort();
        paths.dedup();
        let target_entry = &by_path[&target];
        for path in &paths {
            let member = &by_path[path];
            if member.executable != target_entry.executable
                || member.mode != target_entry.mode
                || member.uid != target_entry.uid
                || member.gid != target_entry.gid
                || member.mtime != target_entry.mtime
            {
                return Err(divergence(format!(
                    "tar hardlink metadata for {path} disagrees with target {target}"
                )));
            }
        }
        let content = sha256_exact(&target_entry.plaintext);
        let group = hardlink_group_id(content, &paths)?;
        for path in paths {
            group_by_path.insert(path, group);
        }
    }

    let mut files = Vec::new();
    for item in by_path.values() {
        let mut metadata_items = vec![
            MetadataItem::executable(item.executable),
            MetadataItem::mtime(item.mtime),
            MetadataItem::posix_mode(item.mode),
            MetadataItem::posix_uid(item.uid),
            MetadataItem::posix_gid(item.gid),
        ];
        if let Some(group) = group_by_path.get(&item.path) {
            metadata_items.push(MetadataItem::hardlink_group(*group));
        }
        let metadata = MetadataSet::new(metadata_items)?;
        match &item.kind {
            ResolvedKind::Directory => {
                entries.push(Entry::new(
                    item.path.clone(),
                    EntryData::Directory,
                    metadata,
                    EntryIdentity::default(),
                ));
            }
            ResolvedKind::File => {
                let digest = sha256_exact(&item.plaintext);
                entries.push(Entry::new(
                    item.path.clone(),
                    EntryData::File {
                        content: ContentRef::Internal(digest),
                    },
                    metadata,
                    EntryIdentity::default(),
                ));
                files.push(item.plaintext.clone());
            }
            ResolvedKind::Hardlink(target) => {
                let target_entry = &by_path[target];
                let digest = sha256_exact(&target_entry.plaintext);
                entries.push(Entry::new(
                    item.path.clone(),
                    EntryData::File {
                        content: ContentRef::Internal(digest),
                    },
                    metadata,
                    EntryIdentity::default(),
                ));
            }
            ResolvedKind::Symlink(target) => {
                entries.push(Entry::new(
                    item.path.clone(),
                    EntryData::Symlink {
                        target: target.clone(),
                    },
                    metadata,
                    EntryIdentity::default(),
                ));
            }
        }
    }
    resolutions.sort();
    resolutions.dedup();
    policy_check(
        u64::try_from(resolutions.len()).unwrap_or(u64::MAX) <= policy.max_resolutions,
        "tar resolutions exceed policy",
    )?;
    let unsupported = observation
        .unsupported_metadata
        .into_iter()
        .collect::<Vec<_>>();
    let provenance = ConversionProvenance {
        source_format: observation.lom.source_format.clone(),
        adapter_id: format!("{}-strict/v1", observation.lom.source_format),
        source_digest: observation.lom.source_digest,
        import_mode: "strict".to_owned(),
        source_entry_count: u64::try_from(observation.entries.len()).unwrap_or(u64::MAX),
        observation_count: observation.lom.observation_count(),
        omission_count: count_resolution(&resolutions, ConflictClass::Omission),
        refinement_count: count_resolution(&resolutions, ConflictClass::Refinement),
        divergence_count: observation.lom.conflict_count(ConflictClass::Divergence),
        irreconcilable_count: observation
            .lom
            .conflict_count(ConflictClass::Irreconcilable),
        resolutions: resolutions.clone().into_boxed_slice(),
        synthesized_ancestors: synthesized
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        unsupported_metadata: unsupported.clone().into_boxed_slice(),
        outcome: "success".to_owned(),
    };
    let archive = plan_observed_archive(
        entries,
        files,
        tar_fidelity(&unsupported),
        provenance,
        None,
        profile,
    )?;
    Ok(TarImportResult {
        archive,
        report: TarConversionReport {
            observation: observation.lom,
            resolutions: resolutions.into_boxed_slice(),
            synthesized_ancestors: synthesized
                .into_iter()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            layers: observation.layers.into_boxed_slice(),
            wrapper_members: observation.wrapper_members,
            decoded_child_digest: observation.decoded_child_digest,
        },
    })
}

pub fn import_strict(
    source: &[u8],
    policy: TarImportPolicy,
    profile: CompressionProfile,
) -> Result<TarImportResult> {
    resolve_strict(observe(source, policy)?, policy, profile)
}

fn resolve_entry(
    entry: &ObservedTarEntry,
    source: &[u8],
    resolutions: &mut Vec<ConversionResolution>,
) -> Result<ResolvedEntry> {
    let header_path = utf8_path(&entry.header_name)?;
    let local_path = entry.local_pax.get("path");
    let global_path = entry.global_pax.get("path");
    if let (Some(pax), Some(gnu)) = (local_path.or(global_path), entry.gnu_long_name.as_ref())
        && pax.value != gnu.value
    {
        return Err(divergence(format!(
            "GNU long name '{}' conflicts with pax path '{}'",
            gnu.value, pax.value
        )));
    }
    let path_text = local_path
        .or(global_path)
        .map(|claim| claim.value.clone())
        .or_else(|| {
            entry
                .gnu_long_name
                .as_ref()
                .map(|claim| claim.value.clone())
        })
        .unwrap_or(header_path);
    let (path, components) = logical_path(&path_text)?;
    let directory_marker = path_text.ends_with('/');
    let kind = match entry.typeflag {
        b'5' => ResolvedKind::Directory,
        0 | b'0' | b'7' => ResolvedKind::File,
        b'1' => ResolvedKind::Hardlink(selected_link_path(entry)?),
        b'2' => ResolvedKind::Symlink(LinkTarget::canonical(selected_link_bytes(entry)?)?),
        b'3' => return Err(unsupported_kind(&path, "character device")),
        b'4' => return Err(unsupported_kind(&path, "block device")),
        b'6' => return Err(unsupported_kind(&path, "FIFO")),
        b'S' => return Err(unsupported_kind(&path, "GNU sparse file")),
        other => return Err(unsupported_kind(&path, &format!("typeflag 0x{other:02x}"))),
    };
    let directory = matches!(kind, ResolvedKind::Directory);
    if directory != directory_marker {
        if directory {
            resolutions.push(ConversionResolution {
                conflict_class: ConflictClass::Omission.as_str().to_owned(),
                semantic_field: format!("entry-kind:{path}"),
                authorities: Box::from(["tar typeflag".to_owned(), "path marker".to_owned()]),
                observed_values: Box::from(["directory".to_owned(), "marker omitted".to_owned()]),
                action: "used explicit directory typeflag".to_owned(),
            });
        } else {
            return Err(divergence(format!(
                "non-directory typeflag conflicts with directory path marker for {path}"
            )));
        }
    }
    let size = selected_u64(
        "size",
        entry.payload_size,
        &entry.global_pax,
        &entry.local_pax,
    )?;
    if directory && size != 0 {
        return Err(irreconcilable(format!(
            "directory {path} has a nonzero tar payload"
        )));
    }
    if (!entry.header_linkname.is_empty()
        || entry.gnu_long_link.is_some()
        || entry.local_pax.contains_key("linkpath"))
        && matches!(entry.typeflag, 0 | b'0' | b'5' | b'7')
    {
        return Err(divergence(format!(
            "non-link tar entry {path} carries link target evidence"
        )));
    }
    let payload_size =
        usize::try_from(size).map_err(|_| structure("selected tar size is not addressable"))?;
    let available = usize::try_from(entry.payload_size).unwrap_or(usize::MAX);
    if payload_size > available {
        return Err(structure(
            "pax-selected tar size exceeds the observed payload extent",
        ));
    }
    if matches!(kind, ResolvedKind::Symlink(_) | ResolvedKind::Hardlink(_)) && size != 0 {
        return Err(irreconcilable(format!(
            "link entry {path} has a nonzero tar payload"
        )));
    }
    let plaintext = if !matches!(kind, ResolvedKind::File) {
        Box::default()
    } else {
        slice(
            source,
            entry.payload_offset,
            payload_size,
            "selected tar payload",
        )?
        .to_vec()
        .into_boxed_slice()
    };
    let mtime = selected_mtime(entry)?;
    let mode =
        u32::try_from(entry.header_mode & 0o7777).map_err(|_| structure("tar mode exceeds u32"))?;
    let uid = u32::try_from(selected_u64(
        "uid",
        entry.header_uid,
        &entry.global_pax,
        &entry.local_pax,
    )?)
    .map_err(|_| structure("tar uid exceeds u32"))?;
    let gid = u32::try_from(selected_u64(
        "gid",
        entry.header_gid,
        &entry.global_pax,
        &entry.local_pax,
    )?)
    .map_err(|_| structure("tar gid exceeds u32"))?;
    Ok(ResolvedEntry {
        path,
        components,
        kind,
        executable: mode & 0o111 != 0,
        mode,
        uid,
        gid,
        mtime,
        plaintext,
    })
}

fn selected_link_bytes(entry: &ObservedTarEntry) -> Result<Box<[u8]>> {
    let pax = entry
        .local_pax
        .get("linkpath")
        .or_else(|| entry.global_pax.get("linkpath"));
    if let (Some(pax), Some(gnu)) = (pax, entry.gnu_long_link.as_ref())
        && pax.value != gnu.value
    {
        return Err(divergence(
            "GNU long-link value conflicts with the normative pax linkpath",
        ));
    }
    if let Some(value) = pax.or(entry.gnu_long_link.as_ref()) {
        Ok(value.value.as_bytes().to_vec().into_boxed_slice())
    } else if entry.header_linkname.is_empty() {
        Err(irreconcilable("tar link entry has no link target"))
    } else {
        Ok(entry.header_linkname.clone())
    }
}

fn selected_link_path(entry: &ObservedTarEntry) -> Result<LogicalPath> {
    let bytes = selected_link_bytes(entry)?;
    let value =
        std::str::from_utf8(&bytes).map_err(|_| unsafe_path("tar hardlink target is not UTF-8"))?;
    logical_path(value).map(|(path, _)| path)
}

fn selected_u64(
    key: &str,
    header: u64,
    global: &BTreeMap<String, PaxClaim>,
    local: &BTreeMap<String, PaxClaim>,
) -> Result<u64> {
    local
        .get(key)
        .or_else(|| global.get(key))
        .map_or(Ok(header), |claim| {
            claim
                .value
                .parse::<u64>()
                .map_err(|_| structure(format!("pax {key} is not a canonical unsigned integer")))
        })
}

fn selected_mtime(entry: &ObservedTarEntry) -> Result<Timestamp> {
    if let Some(claim) = entry
        .local_pax
        .get("mtime")
        .or_else(|| entry.global_pax.get("mtime"))
    {
        let (seconds, nanos, precision) = parse_pax_time(&claim.value)?;
        Timestamp::new(seconds, nanos, precision, true)
    } else {
        Timestamp::new(entry.header_mtime, 0, TimestampPrecision::Second, true)
    }
}

fn parse_pax_time(value: &str) -> Result<(i64, u32, TimestampPrecision)> {
    let (whole, fractional) = value.split_once('.').unwrap_or((value, ""));
    let mut seconds = whole
        .parse::<i64>()
        .map_err(|_| structure("pax mtime seconds are invalid"))?;
    if fractional.is_empty() {
        return Ok((seconds, 0, TimestampPrecision::Second));
    }
    if fractional.len() > 9 || !fractional.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(structure(
            "pax mtime fractional precision exceeds nanoseconds",
        ));
    }
    let mut nanos = fractional.parse::<u32>().unwrap();
    nanos *= 10_u32.pow(u32::try_from(9 - fractional.len()).unwrap());
    if value.starts_with('-') && nanos != 0 {
        seconds = seconds
            .checked_sub(1)
            .ok_or_else(|| structure("pax mtime underflows"))?;
        nanos = 1_000_000_000 - nanos;
    }
    let precision = match fractional.len() {
        1..=2 => TimestampPrecision::Centisecond,
        3..=6 => TimestampPrecision::Microsecond,
        7 => TimestampPrecision::Hectonanosecond,
        _ => TimestampPrecision::Nanosecond,
    };
    Ok((seconds, nanos, precision))
}

fn classify_overrides(
    header: HeaderClaims<'_>,
    global: &BTreeMap<String, PaxClaim>,
    local: &BTreeMap<String, PaxClaim>,
    gnu: Option<&PaxClaim>,
    conflicts: &mut Vec<LegacyConflict>,
) {
    let header_path = String::from_utf8_lossy(header.path).into_owned();
    if let Some(global_claim) = global.get("path")
        && global_claim.value != header_path
    {
        conflicts.push(conflict_with_header(
            "path",
            header.authority,
            header.location,
            &header_path,
            global_claim,
            "POSIX global pax path overrides the base header",
        ));
    }
    if let Some(local_claim) = local.get("path") {
        if let Some(global_claim) = global.get("path")
            && local_claim.value != global_claim.value
        {
            conflicts.push(conflict_claims(
                "path",
                global_claim,
                local_claim,
                ConflictClass::Refinement,
                Some(LegacyResolution {
                    action: "POSIX per-entry pax path overrides global pax path".to_owned(),
                    selected_authority: Some(local_claim.authority.clone()),
                }),
            ));
        } else if global.get("path").is_none() && local_claim.value != header_path {
            conflicts.push(conflict_with_header(
                "path",
                header.authority,
                header.location,
                &header_path,
                local_claim,
                "POSIX per-entry pax path overrides prior path authority",
            ));
        }
    } else if let Some(gnu_claim) = gnu
        && global.get("path").is_none()
        && gnu_claim.value != header_path
    {
        conflicts.push(conflict_with_header(
            "path",
            header.authority,
            header.location,
            &header_path,
            gnu_claim,
            "GNU long-name record refines the base header name",
        ));
    }
    for (key, header_value) in [
        ("size", header.size.to_string()),
        ("mtime", header.mtime.to_string()),
        ("uid", header.uid.to_string()),
        ("gid", header.gid.to_string()),
    ] {
        if let Some(selected) = local.get(key).or_else(|| global.get(key))
            && selected.value != header_value
        {
            conflicts.push(conflict_with_header(
                key,
                header.authority,
                header.location,
                &header_value,
                selected,
                &format!("POSIX pax {key} overrides the base header"),
            ));
        }
    }
    for key in ["size", "mtime", "uid", "gid", "uname", "gname"] {
        if let (Some(global_claim), Some(local_claim)) = (global.get(key), local.get(key))
            && global_claim.value != local_claim.value
        {
            conflicts.push(conflict_claims(
                key,
                global_claim,
                local_claim,
                ConflictClass::Refinement,
                Some(LegacyResolution {
                    action: format!("POSIX per-entry pax {key} overrides global pax {key}"),
                    selected_authority: Some(local_claim.authority.clone()),
                }),
            ));
        }
    }
}

fn parse_pax(
    payload: &[u8],
    payload_offset: usize,
    authority: &LegacyAuthority,
    conflicts: &mut Vec<LegacyConflict>,
    record_count: &mut u64,
    policy: TarImportPolicy,
) -> Result<ParsedPax> {
    let mut cursor = 0_usize;
    let mut values = BTreeMap::<String, PaxClaim>::new();
    let mut fields = Vec::new();
    while cursor < payload.len() {
        *record_count = record_count.saturating_add(1);
        policy_check(
            *record_count <= policy.max_pax_records,
            "pax record count exceeds policy",
        )?;
        let space = payload[cursor..]
            .iter()
            .position(|byte| *byte == b' ')
            .ok_or_else(|| structure("pax record length has no separating space"))?;
        if space == 0 || payload[cursor] == b'0' {
            return Err(structure("pax record length is not canonical"));
        }
        let length = std::str::from_utf8(&payload[cursor..cursor + space])
            .map_err(|_| structure("pax record length is not ASCII"))?
            .parse::<usize>()
            .map_err(|_| structure("pax record length is invalid"))?;
        let end = cursor
            .checked_add(length)
            .ok_or_else(|| structure("pax record extent overflow"))?;
        let record = payload
            .get(cursor..end)
            .ok_or_else(|| structure("pax record is truncated"))?;
        if record.last() != Some(&b'\n') || length <= space + 3 {
            return Err(structure("pax record framing is invalid"));
        }
        let body = &record[space + 1..record.len() - 1];
        let equals = body
            .iter()
            .position(|byte| *byte == b'=')
            .ok_or_else(|| structure("pax record omits '='"))?;
        let key =
            std::str::from_utf8(&body[..equals]).map_err(|_| structure("pax key is not UTF-8"))?;
        let value = std::str::from_utf8(&body[equals + 1..])
            .map_err(|_| structure("pax value is not UTF-8"))?
            .to_owned();
        if key.is_empty() || key.bytes().any(|byte| byte == b'=' || byte == b'\n') {
            return Err(structure("pax key is invalid"));
        }
        let claim = PaxClaim {
            value: value.clone(),
            authority: authority.clone(),
            location: location(payload_offset + cursor, length),
        };
        if let Some(previous) = values.insert(key.to_owned(), claim.clone()) {
            let class = if previous.value == claim.value {
                ConflictClass::Refinement
            } else {
                ConflictClass::Divergence
            };
            conflicts.push(conflict_claims(
                key,
                &previous,
                &claim,
                class,
                (class == ConflictClass::Refinement).then_some(LegacyResolution {
                    action: "identical repeated pax key retained once".to_owned(),
                    selected_authority: Some(claim.authority.clone()),
                }),
            ));
        }
        fields.push(field_text(
            &format!("pax.{key}"),
            authority,
            &value,
            payload_offset + cursor,
            length,
        ));
        cursor = end;
    }
    Ok((values, fields))
}

fn known_pax_key(key: &str) -> bool {
    matches!(
        key,
        "path" | "size" | "mtime" | "uid" | "gid" | "uname" | "gname" | "linkpath"
    )
}

fn merge_pax(
    target: &mut BTreeMap<String, PaxClaim>,
    incoming: BTreeMap<String, PaxClaim>,
    global: bool,
    conflicts: &mut Vec<LegacyConflict>,
) -> Result<()> {
    for (key, claim) in incoming {
        if let Some(previous) = target.insert(key.clone(), claim.clone())
            && previous.value != claim.value
        {
            conflicts.push(conflict_claims(
                &key,
                &previous,
                &claim,
                ConflictClass::Refinement,
                Some(LegacyResolution {
                    action: format!(
                        "later {} pax header overrides earlier value",
                        if global { "global" } else { "per-entry" }
                    ),
                    selected_authority: Some(claim.authority.clone()),
                }),
            ));
        }
    }
    Ok(())
}

fn pax_fields(
    values: &BTreeMap<String, PaxClaim>,
) -> Vec<LegacyFieldObservation<LegacyObservedValue>> {
    values
        .iter()
        .map(|(key, claim)| {
            field_text(
                &format!("pax.{key}"),
                &claim.authority,
                &claim.value,
                usize::try_from(claim.location.offset).unwrap_or(0),
                usize::try_from(claim.location.length).unwrap_or(0),
            )
        })
        .collect()
}

fn refuse_unresolved(conflicts: &[LegacyConflict]) -> Result<()> {
    if let Some(conflict) = conflicts.iter().find(|conflict| {
        matches!(
            conflict.classification,
            ConflictClass::Divergence | ConflictClass::Irreconcilable
        ) && conflict.resolution.is_none()
    }) {
        return Err(match conflict.classification {
            ConflictClass::Divergence => divergence(format!(
                "{} has divergent tar authorities",
                conflict.semantic_field
            )),
            ConflictClass::Irreconcilable => {
                irreconcilable(format!("{} is irreconcilable", conflict.semantic_field))
            }
            _ => unreachable!(),
        });
    }
    Ok(())
}

fn conflict_resolution(conflict: &LegacyConflict) -> Option<ConversionResolution> {
    conflict
        .resolution
        .as_ref()
        .map(|resolution| ConversionResolution {
            conflict_class: conflict.classification.as_str().to_owned(),
            semantic_field: conflict.semantic_field.clone(),
            authorities: conflict
                .authorities
                .iter()
                .map(authority_name)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            observed_values: conflict
                .observed_values
                .iter()
                .map(LegacyObservedValue::display_compact)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            action: resolution.action.clone(),
        })
}

fn logical_path(value: &str) -> Result<(LogicalPath, Vec<String>)> {
    let trimmed = value.strip_suffix('/').unwrap_or(value);
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.contains('\\')
        || trimmed.as_bytes().contains(&0)
        || trimmed
            .as_bytes()
            .get(1)
            .is_some_and(|value| *value == b':')
    {
        return Err(unsafe_path(format!("unsafe tar path '{value}'")));
    }
    let components = trimmed.split('/').map(str::to_owned).collect::<Vec<_>>();
    if components
        .iter()
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(unsafe_path(format!("unsafe tar path '{value}'")));
    }
    LogicalPath::from_utf8(&components)
        .map(|path| (path, components))
        .map_err(|error| unsafe_path(error.detail()))
}

fn utf8_path(value: &[u8]) -> Result<String> {
    std::str::from_utf8(value)
        .map(str::to_owned)
        .map_err(|_| unsafe_path("tar path is not unambiguous UTF-8"))
}

fn joined_header_name(prefix: &[u8], name: &[u8], max: u64) -> Result<Box<[u8]>> {
    let mut result = Vec::new();
    if !prefix.is_empty() {
        result.extend_from_slice(prefix);
        result.push(b'/');
    }
    result.extend_from_slice(name);
    policy_check(
        u64::try_from(result.len()).unwrap_or(u64::MAX) <= max,
        "tar name exceeds policy",
    )?;
    Ok(result.into_boxed_slice())
}

fn checksum_valid(header: &[u8]) -> bool {
    if header.len() != BLOCK {
        return false;
    }
    let Ok(expected) = unsigned_number(&header[148..156], "tar checksum") else {
        return false;
    };
    let unsigned = header
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            if (148..156).contains(&index) {
                32_u64
            } else {
                u64::from(*byte)
            }
        })
        .sum::<u64>();
    let signed = header
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            if (148..156).contains(&index) {
                32_i64
            } else {
                i64::from(i8::from_ne_bytes([*byte]))
            }
        })
        .sum::<i64>();
    expected == unsigned || i64::try_from(expected).ok() == Some(signed)
}

fn unsigned_number(bytes: &[u8], what: &str) -> Result<u64> {
    let value = signed_number(bytes, what)?;
    u64::try_from(value).map_err(|_| structure(format!("{what} is negative")))
}

fn signed_number(bytes: &[u8], what: &str) -> Result<i64> {
    if bytes.is_empty() {
        return Err(structure(format!("{what} is empty")));
    }
    if bytes[0] & 0x80 != 0 {
        let negative = bytes[0] & 0x40 != 0;
        let mut value = if negative { -1_i128 } else { 0_i128 };
        for (index, byte) in bytes.iter().enumerate() {
            let byte = if index == 0 { byte & 0x7f } else { *byte };
            value = (value << 8) | i128::from(byte);
        }
        return i64::try_from(value).map_err(|_| structure(format!("{what} overflows i64")));
    }
    let trimmed = bytes
        .iter()
        .copied()
        .skip_while(|byte| matches!(*byte, 0 | b' '))
        .take_while(|byte| *byte != 0 && *byte != b' ')
        .collect::<Vec<_>>();
    if trimmed.is_empty() {
        return Ok(0);
    }
    if !trimmed.iter().all(|byte| matches!(*byte, b'0'..=b'7')) {
        return Err(structure(format!("{what} is not octal or base-256")));
    }
    let text = std::str::from_utf8(&trimmed).unwrap();
    i64::from_str_radix(text, 8).map_err(|_| structure(format!("{what} overflows i64")))
}

fn round_block(value: usize) -> Result<usize> {
    value
        .checked_add(BLOCK - 1)
        .map(|value| value / BLOCK * BLOCK)
        .ok_or_else(|| structure("tar padded extent overflow"))
}

fn string_field(header: &[u8], offset: usize, length: usize) -> Box<[u8]> {
    trim_nul(&header[offset..offset + length])
        .to_vec()
        .into_boxed_slice()
}

fn trim_nul(value: &[u8]) -> &[u8] {
    let end = value
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(value.len());
    &value[..end]
}

fn tar_fidelity(unsupported: &[String]) -> FidelityReport {
    FidelityReport {
        captured: Box::from([
            "core.executable".to_owned(),
            "core.mtime".to_owned(),
            "posix.mode".to_owned(),
            "posix.uid".to_owned(),
            "posix.gid".to_owned(),
            "posix.hardlink-group".to_owned(),
            "symlink-target".to_owned(),
            "legacy.conversion-provenance".to_owned(),
        ]),
        unavailable: unsupported
            .iter()
            .map(|class| FidelityIssue {
                class: class.clone(),
                reason:
                    "observed as tar/pax evidence but unsupported by the POSIX metadata-v1 subset"
                        .to_owned(),
                entry_scope: None,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        degraded: Box::default(),
        platform: "legacy:tar".to_owned(),
        filesystem: Box::default(),
    }
}

fn count_resolution(resolutions: &[ConversionResolution], class: ConflictClass) -> u64 {
    resolutions
        .iter()
        .filter(|resolution| resolution.conflict_class == class.as_str())
        .count()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn conflict_with_header(
    field: &str,
    header_authority: &LegacyAuthority,
    header_location: LegacyEvidenceLocation,
    header_value: &str,
    selected: &PaxClaim,
    action: &str,
) -> LegacyConflict {
    LegacyConflict {
        semantic_field: field.to_owned(),
        authorities: Box::from([header_authority.clone(), selected.authority.clone()]),
        observed_values: Box::from([
            LegacyObservedValue::Text(header_value.to_owned()),
            LegacyObservedValue::Text(selected.value.clone()),
        ]),
        evidence: Box::from([header_location, selected.location]),
        classification: ConflictClass::Refinement,
        resolution: Some(LegacyResolution {
            action: action.to_owned(),
            selected_authority: Some(selected.authority.clone()),
        }),
    }
}

fn conflict_claims(
    field: &str,
    left: &PaxClaim,
    right: &PaxClaim,
    classification: ConflictClass,
    resolution: Option<LegacyResolution>,
) -> LegacyConflict {
    LegacyConflict {
        semantic_field: field.to_owned(),
        authorities: Box::from([left.authority.clone(), right.authority.clone()]),
        observed_values: Box::from([
            LegacyObservedValue::Text(left.value.clone()),
            LegacyObservedValue::Text(right.value.clone()),
        ]),
        evidence: Box::from([left.location, right.location]),
        classification,
        resolution,
    }
}

fn conflict(
    field: &str,
    left: &PaxClaim,
    right: &PaxClaim,
    classification: ConflictClass,
    resolution: Option<LegacyResolution>,
) -> LegacyConflict {
    conflict_claims(field, left, right, classification, resolution)
}

fn authority(structure: &str, instance: u64) -> LegacyAuthority {
    LegacyAuthority {
        format: "tar".to_owned(),
        structure: structure.to_owned(),
        instance,
    }
}

fn authority_name(authority: &LegacyAuthority) -> String {
    format!(
        "{}:{}:{}",
        authority.format, authority.structure, authority.instance
    )
}

fn field_u64(
    semantic_field: &str,
    authority: &LegacyAuthority,
    value: u64,
    offset: usize,
    length: usize,
) -> LegacyFieldObservation<LegacyObservedValue> {
    LegacyFieldObservation {
        semantic_field: semantic_field.to_owned(),
        authority: authority.clone(),
        raw_value: value.to_be_bytes().to_vec().into_boxed_slice(),
        interpreted_value: Some(LegacyObservedValue::Unsigned(value)),
        evidence: location(offset, length),
        validity: ObservationValidity::Valid,
    }
}

fn field_i64(
    semantic_field: &str,
    authority: &LegacyAuthority,
    value: i64,
    offset: usize,
    length: usize,
) -> LegacyFieldObservation<LegacyObservedValue> {
    LegacyFieldObservation {
        semantic_field: semantic_field.to_owned(),
        authority: authority.clone(),
        raw_value: value.to_be_bytes().to_vec().into_boxed_slice(),
        interpreted_value: Some(LegacyObservedValue::Signed(value)),
        evidence: location(offset, length),
        validity: ObservationValidity::Valid,
    }
}

fn field_bytes(
    semantic_field: &str,
    authority: &LegacyAuthority,
    value: &[u8],
    offset: usize,
) -> LegacyFieldObservation<LegacyObservedValue> {
    LegacyFieldObservation {
        semantic_field: semantic_field.to_owned(),
        authority: authority.clone(),
        raw_value: value.to_vec().into_boxed_slice(),
        interpreted_value: Some(LegacyObservedValue::Bytes(
            value.to_vec().into_boxed_slice(),
        )),
        evidence: location(offset, value.len()),
        validity: ObservationValidity::Valid,
    }
}

fn field_text(
    semantic_field: &str,
    authority: &LegacyAuthority,
    value: &str,
    offset: usize,
    length: usize,
) -> LegacyFieldObservation<LegacyObservedValue> {
    LegacyFieldObservation {
        semantic_field: semantic_field.to_owned(),
        authority: authority.clone(),
        raw_value: value.as_bytes().to_vec().into_boxed_slice(),
        interpreted_value: Some(LegacyObservedValue::Text(value.to_owned())),
        evidence: location(offset, length),
        validity: ObservationValidity::Valid,
    }
}

fn location(offset: usize, length: usize) -> LegacyEvidenceLocation {
    LegacyEvidenceLocation {
        offset: u64::try_from(offset).unwrap_or(u64::MAX),
        length: u64::try_from(length).unwrap_or(u64::MAX),
    }
}

fn slice<'a>(source: &'a [u8], offset: usize, length: usize, what: &str) -> Result<&'a [u8]> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| structure(format!("{what} extent overflow")))?;
    source
        .get(offset..end)
        .ok_or_else(|| structure(format!("{what} is truncated")))
}

fn policy_check(condition: bool, detail: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(policy_error(detail))
    }
}

fn structure(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::TarStructureInvalid,
        detail,
    )
}

fn divergence(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::TarConflictDivergence,
        detail,
    )
}

fn irreconcilable(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::TarConflictIrreconcilable,
        detail,
    )
}

fn unsafe_path(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::TarUnsafePath,
        detail,
    )
}

fn unsupported_kind(path: &LogicalPath, kind: &str) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Unsupported,
        ReasonCode::TarUnsupportedFeature,
        format!("tar entry {path} uses unsupported {kind}"),
    )
}

fn policy_error(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::LegacyResourcePolicyRefused,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecf::{WriteOptions, encode, open};

    struct TestEntry<'a> {
        name: &'a [u8],
        content: &'a [u8],
        typeflag: u8,
        mode: u64,
        prefix: &'a [u8],
        base256_size: bool,
    }

    fn octal(field: &mut [u8], value: u64) {
        field.fill(0);
        let text = format!("{:0width$o}", value, width = field.len() - 1);
        field[..text.len()].copy_from_slice(text.as_bytes());
    }

    fn append_entry(output: &mut Vec<u8>, entry: &TestEntry<'_>) {
        let mut header = [0_u8; BLOCK];
        header[..entry.name.len()].copy_from_slice(entry.name);
        octal(&mut header[100..108], entry.mode);
        octal(&mut header[108..116], 1000);
        octal(&mut header[116..124], 1000);
        if entry.base256_size {
            let value = u64::try_from(entry.content.len()).unwrap();
            header[124] = 0x80;
            header[128..136].copy_from_slice(&value.to_be_bytes());
        } else {
            octal(
                &mut header[124..136],
                u64::try_from(entry.content.len()).unwrap(),
            );
        }
        octal(&mut header[136..148], 1_700_000_000);
        header[148..156].fill(b' ');
        header[156] = entry.typeflag;
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        header[345..345 + entry.prefix.len()].copy_from_slice(entry.prefix);
        let checksum = header.iter().map(|byte| u64::from(*byte)).sum::<u64>();
        let text = format!("{checksum:06o}\0 ");
        header[148..156].copy_from_slice(text.as_bytes());
        output.extend_from_slice(&header);
        output.extend_from_slice(entry.content);
        output.resize(output.len().div_ceil(BLOCK) * BLOCK, 0);
    }

    fn archive(entries: &[TestEntry<'_>]) -> Vec<u8> {
        let mut output = Vec::new();
        for entry in entries {
            append_entry(&mut output, entry);
        }
        output.resize(output.len() + BLOCK * 2, 0);
        output
    }

    fn set_linkname(source: &mut [u8], entry_name: &[u8], linkname: &[u8]) {
        let mut cursor = 0;
        loop {
            let header = &mut source[cursor..cursor + BLOCK];
            let name_end = header[..100]
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(100);
            let size = unsigned_number(&header[124..136], "test size").unwrap();
            if &header[..name_end] == entry_name {
                header[157..257].fill(0);
                header[157..157 + linkname.len()].copy_from_slice(linkname);
                header[148..156].fill(b' ');
                let checksum = header.iter().map(|byte| u64::from(*byte)).sum::<u64>();
                let text = format!("{checksum:06o}\0 ");
                header[148..156].copy_from_slice(text.as_bytes());
                return;
            }
            cursor += BLOCK + usize::try_from(size).unwrap().div_ceil(BLOCK) * BLOCK;
        }
    }

    fn pax(key: &str, value: &str) -> Vec<u8> {
        let body = format!("{key}={value}\n");
        let mut length = body.len() + 2;
        loop {
            let record = format!("{length} {body}");
            if record.len() == length {
                return record.into_bytes();
            }
            length = record.len();
        }
    }

    #[test]
    fn ustar_prefix_ancestors_metadata_and_base256_round_trip() {
        let source = archive(&[
            TestEntry {
                name: b"dir",
                content: b"",
                typeflag: b'5',
                mode: 0o755,
                prefix: b"",
                base256_size: false,
            },
            TestEntry {
                name: b"tool",
                content: b"payload",
                typeflag: b'0',
                mode: 0o755,
                prefix: b"dir/sub",
                base256_size: true,
            },
        ]);
        let imported = import_strict(
            &source,
            TarImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        assert_eq!(imported.report.synthesized_ancestors.len(), 1);
        assert!(
            imported
                .archive
                .entry_set
                .entries
                .iter()
                .any(|entry| entry.path().to_string() == "dir/sub/tool")
        );
        let encoded = encode(&imported.archive, WriteOptions::default()).unwrap();
        let opened = open(&encoded.bytes).unwrap();
        assert_eq!(opened.archive.entry_set, encoded.archive.entry_set);
    }

    #[test]
    fn pax_and_gnu_overrides_are_observed_and_resolved() {
        let pax_data = [pax("path", "pax/renamed"), pax("mtime", "1700000000.25")].concat();
        let source = archive(&[
            TestEntry {
                name: b"PaxHeader",
                content: &pax_data,
                typeflag: b'x',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            },
            TestEntry {
                name: b"short",
                content: b"value",
                typeflag: b'0',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            },
            TestEntry {
                name: b"LongLink",
                content: b"gnu/name\0",
                typeflag: b'L',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            },
            TestEntry {
                name: b"ignored",
                content: b"gnu",
                typeflag: b'0',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            },
        ]);
        let imported = import_strict(
            &source,
            TarImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        let paths = imported
            .archive
            .entry_set
            .entries
            .iter()
            .map(|entry| entry.path().to_string())
            .collect::<Vec<_>>();
        assert!(paths.contains(&"pax/renamed".to_owned()));
        assert!(paths.contains(&"gnu/name".to_owned()));
        assert!(
            imported
                .report
                .resolutions
                .iter()
                .any(|resolution| resolution.conflict_class == "refinement")
        );
    }

    #[test]
    fn strict_tar_rejects_checksum_paths_links_conflicts_and_trailing_garbage() {
        let base = || {
            archive(&[TestEntry {
                name: b"file",
                content: b"value",
                typeflag: b'0',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            }])
        };
        let mut checksum = base();
        checksum[0] ^= 1;
        assert_eq!(
            observe(&checksum, TarImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::TarChecksumMismatch
        );

        let unsafe_source = archive(&[TestEntry {
            name: b"../escape",
            content: b"x",
            typeflag: b'0',
            mode: 0o644,
            prefix: b"",
            base256_size: false,
        }]);
        assert_eq!(
            import_strict(
                &unsafe_source,
                TarImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::TarUnsafePath
        );

        let symlink = archive(&[TestEntry {
            name: b"link",
            content: b"",
            typeflag: b'2',
            mode: 0o777,
            prefix: b"",
            base256_size: false,
        }]);
        assert_eq!(
            import_strict(
                &symlink,
                TarImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::TarConflictIrreconcilable
        );

        let mut garbage = base();
        let end = garbage.len() - BLOCK * 2;
        garbage[end + BLOCK * 2 - 1] = 1;
        assert_eq!(
            observe(&garbage, TarImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::TarStructureInvalid
        );
    }

    #[test]
    fn symlink_and_hardlink_project_to_native_posix_semantics() {
        let mut source = archive(&[
            TestEntry {
                name: b"target",
                content: b"shared bytes",
                typeflag: b'0',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            },
            TestEntry {
                name: b"alias",
                content: b"",
                typeflag: b'1',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            },
            TestEntry {
                name: b"link",
                content: b"",
                typeflag: b'2',
                mode: 0o777,
                prefix: b"",
                base256_size: false,
            },
        ]);
        set_linkname(&mut source, b"alias", b"target");
        set_linkname(&mut source, b"link", b"../outside-is-semantic-data");
        let imported = import_strict(
            &source,
            TarImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        let target = imported
            .archive
            .entry_set
            .entries()
            .iter()
            .find(|entry| entry.path().to_string() == "target")
            .unwrap();
        let alias = imported
            .archive
            .entry_set
            .entries()
            .iter()
            .find(|entry| entry.path().to_string() == "alias")
            .unwrap();
        assert_eq!(target.data(), alias.data());
        assert_eq!(
            target.metadata().hardlink_group(),
            alias.metadata().hardlink_group()
        );
        assert!(target.metadata().hardlink_group().is_some());
        let link = imported
            .archive
            .entry_set
            .entries()
            .iter()
            .find(|entry| entry.path().to_string() == "link")
            .unwrap();
        let EntryData::Symlink { target } = link.data() else {
            panic!("tar symlink must remain a native Symlink Entry");
        };
        assert_eq!(target.bytes(), b"../outside-is-semantic-data");

        let encoded = encode(&imported.archive, WriteOptions::default()).unwrap();
        assert_eq!(
            open(&encoded.bytes).unwrap().archive.entry_set,
            imported.archive.entry_set
        );
    }

    #[test]
    fn conflicting_pax_and_gnu_paths_fail_closed() {
        let pax_data = pax("path", "pax/name");
        let source = archive(&[
            TestEntry {
                name: b"PaxHeader",
                content: &pax_data,
                typeflag: b'x',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            },
            TestEntry {
                name: b"LongLink",
                content: b"gnu/name\0",
                typeflag: b'L',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            },
            TestEntry {
                name: b"ignored",
                content: b"x",
                typeflag: b'0',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            },
        ]);
        assert_eq!(
            import_strict(
                &source,
                TarImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::TarConflictDivergence
        );
    }

    #[test]
    fn empty_duplicate_ancestor_extension_and_special_cases_are_bounded() {
        let empty = archive(&[]);
        assert!(
            import_strict(&empty, TarImportPolicy::default(), CompressionProfile::Fast).is_ok()
        );

        let duplicate = archive(&[
            TestEntry {
                name: b"same",
                content: b"one",
                typeflag: b'0',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            },
            TestEntry {
                name: b"same",
                content: b"two",
                typeflag: b'0',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            },
        ]);
        assert_eq!(
            import_strict(
                &duplicate,
                TarImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::DuplicateLogicalPath
        );

        let file_ancestor = archive(&[
            TestEntry {
                name: b"node",
                content: b"file",
                typeflag: b'0',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            },
            TestEntry {
                name: b"node/child",
                content: b"child",
                typeflag: b'0',
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            },
        ]);
        assert_eq!(
            import_strict(
                &file_ancestor,
                TarImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::FileAsAncestor
        );

        let malformed_pax = archive(&[TestEntry {
            name: b"PaxHeader",
            content: b"12 path=x\n",
            typeflag: b'x',
            mode: 0o644,
            prefix: b"",
            base256_size: false,
        }]);
        assert_eq!(
            observe(&malformed_pax, TarImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::TarStructureInvalid
        );

        for typeflag in *b"346S" {
            let special = archive(&[TestEntry {
                name: b"special",
                content: b"",
                typeflag,
                mode: 0o644,
                prefix: b"",
                base256_size: false,
            }]);
            assert_eq!(
                import_strict(
                    &special,
                    TarImportPolicy::default(),
                    CompressionProfile::Fast
                )
                .unwrap_err()
                .code(),
                ReasonCode::TarUnsupportedFeature
            );
        }
        let missing_hardlink_target = archive(&[TestEntry {
            name: b"hardlink",
            content: b"",
            typeflag: b'1',
            mode: 0o644,
            prefix: b"",
            base256_size: false,
        }]);
        assert_eq!(
            import_strict(
                &missing_hardlink_target,
                TarImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::TarConflictIrreconcilable
        );
    }
}
