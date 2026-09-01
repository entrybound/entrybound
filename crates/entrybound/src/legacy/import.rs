//! Format-neutral strict legacy import dispatch and wrapper composition.

use std::str::FromStr;

use super::{stream, tar, zip};
use crate::archive::plan_observed_archive;
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ContentRef, ConversionProvenance, Entry, EntryData, EntryIdentity, FidelityIssue,
    FidelityReport, LogicalPath, MetadataItem, MetadataSet,
};
use crate::identity::sha256_exact;
use crate::legacy::lom::LegacyArchiveObservation;
use crate::planner::CompressionProfile;

/// Explicit or detected foreign source grammar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacySourceFormat {
    Zip,
    Tar,
    Gzip,
    Zstandard,
    Xz,
    Bzip2,
    TarGzip,
    TarZstandard,
    TarXz,
    TarBzip2,
}

impl LegacySourceFormat {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::Tar => "tar",
            Self::Gzip => "gzip",
            Self::Zstandard => "zstd",
            Self::Xz => "xz",
            Self::Bzip2 => "bzip2",
            Self::TarGzip => "tar.gz",
            Self::TarZstandard => "tar.zst",
            Self::TarXz => "tar.xz",
            Self::TarBzip2 => "tar.bz2",
        }
    }

    const fn transport(self) -> Option<stream::TransportFormat> {
        match self {
            Self::Gzip | Self::TarGzip => Some(stream::TransportFormat::Gzip),
            Self::Zstandard | Self::TarZstandard => Some(stream::TransportFormat::Zstandard),
            Self::Xz | Self::TarXz => Some(stream::TransportFormat::Xz),
            Self::Bzip2 | Self::TarBzip2 => Some(stream::TransportFormat::Bzip2),
            Self::Zip | Self::Tar => None,
        }
    }

    const fn requires_tar_child(self) -> bool {
        matches!(
            self,
            Self::TarGzip | Self::TarZstandard | Self::TarXz | Self::TarBzip2
        )
    }
}

impl FromStr for LegacySourceFormat {
    type Err = Diagnostic;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "zip" => Ok(Self::Zip),
            "tar" => Ok(Self::Tar),
            "gzip" => Ok(Self::Gzip),
            "zstd" => Ok(Self::Zstandard),
            "xz" => Ok(Self::Xz),
            "bzip2" => Ok(Self::Bzip2),
            "tar.gz" => Ok(Self::TarGzip),
            "tar.zst" => Ok(Self::TarZstandard),
            "tar.xz" => Ok(Self::TarXz),
            "tar.bz2" => Ok(Self::TarBzip2),
            _ => Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::LegacyFormatUnsupported,
                format!("unsupported legacy source format '{value}'"),
            )),
        }
    }
}

/// All strict legacy limits, grouped without merging format-specific rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct LegacyImportPolicy {
    pub zip: zip::ZipImportPolicy,
    pub tar: tar::TarImportPolicy,
    pub wrapper: stream::WrapperImportPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyConversionReport {
    pub observation: LegacyArchiveObservation,
    pub synthesized_ancestors: Box<[LogicalPath]>,
    pub layers: Box<[String]>,
    pub wrapper_members: u64,
    pub decoded_child_digest: Option<crate::eam::Digest>,
    pub projection: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyImportResult {
    pub archive: Archive,
    pub report: LegacyConversionReport,
}

#[must_use]
pub fn detect(source: &[u8]) -> Option<LegacySourceFormat> {
    if source.starts_with(b"PK\x03\x04")
        || source.starts_with(b"PK\x05\x06")
        || source.starts_with(b"PK\x07\x08")
    {
        Some(LegacySourceFormat::Zip)
    } else if let Some(format) = stream::detect(source) {
        Some(match format {
            stream::TransportFormat::Gzip => LegacySourceFormat::Gzip,
            stream::TransportFormat::Zstandard => LegacySourceFormat::Zstandard,
            stream::TransportFormat::Xz => LegacySourceFormat::Xz,
            stream::TransportFormat::Bzip2 => LegacySourceFormat::Bzip2,
        })
    } else if tar::looks_like_tar(source) {
        Some(LegacySourceFormat::Tar)
    } else if source[source.len().saturating_sub(65_557)..]
        .windows(4)
        .any(|window| window == b"PK\x05\x06")
    {
        Some(LegacySourceFormat::Zip)
    } else {
        None
    }
}

pub fn import_strict(
    source: &[u8],
    explicit_format: Option<LegacySourceFormat>,
    entry_name: Option<&str>,
    policy: LegacyImportPolicy,
    profile: CompressionProfile,
) -> Result<LegacyImportResult> {
    let detected = detect(source).ok_or_else(|| {
        Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::LegacyFormatUnsupported,
            "source is not structurally recognizable as ZIP, tar, gzip, Zstandard, XZ, or bzip2",
        )
    })?;
    let selected = explicit_format.unwrap_or(detected);
    match selected {
        LegacySourceFormat::Zip => {
            if detected != LegacySourceFormat::Zip {
                return Err(format_mismatch(selected, detected));
            }
            if entry_name.is_some() {
                return Err(usage(
                    "--entry-name is only valid for standalone compressed streams",
                ));
            }
            let imported = zip::import_strict(source, policy.zip, profile)?;
            Ok(LegacyImportResult {
                report: LegacyConversionReport {
                    observation: imported.report.observation,
                    synthesized_ancestors: imported.report.synthesized_ancestors,
                    layers: Box::from(["zip".to_owned()]),
                    wrapper_members: 0,
                    decoded_child_digest: None,
                    projection: "archive".to_owned(),
                },
                archive: imported.archive,
            })
        }
        LegacySourceFormat::Tar => {
            if detected != LegacySourceFormat::Tar {
                return Err(format_mismatch(selected, detected));
            }
            if entry_name.is_some() {
                return Err(usage(
                    "--entry-name is only valid for standalone compressed streams",
                ));
            }
            map_tar(tar::import_strict(source, policy.tar, profile)?)
        }
        wrapper_format => {
            let expected_transport = wrapper_format.transport().expect("wrapper format");
            if stream::detect(source) != Some(expected_transport) {
                return Err(format_mismatch(selected, detected));
            }
            let transport = stream::decode(source, expected_transport, policy.wrapper)?;
            if tar::looks_like_tar(&transport.decoded) {
                if entry_name.is_some() {
                    return Err(usage(
                        "--entry-name cannot replace a structurally valid decoded tar archive",
                    ));
                }
                let mut observation = tar::observe(&transport.decoded, policy.tar)?;
                observation.attach_transport(&transport);
                map_tar(tar::resolve_strict(observation, policy.tar, profile)?)
            } else {
                if wrapper_format.requires_tar_child() {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::TarStructureInvalid,
                        format!(
                            "{} decoded child is not a strict tar archive",
                            selected.as_str()
                        ),
                    ));
                }
                let name = entry_name.ok_or_else(|| {
                    usage("a standalone compressed stream requires --entry-name <logical-path>")
                })?;
                standalone_transport(transport, name, profile)
            }
        }
    }
}

fn map_tar(imported: tar::TarImportResult) -> Result<LegacyImportResult> {
    Ok(LegacyImportResult {
        report: LegacyConversionReport {
            observation: imported.report.observation,
            synthesized_ancestors: imported.report.synthesized_ancestors,
            layers: imported.report.layers,
            wrapper_members: imported.report.wrapper_members,
            decoded_child_digest: imported.report.decoded_child_digest,
            projection: "archive".to_owned(),
        },
        archive: imported.archive,
    })
}

fn standalone_transport(
    transport: stream::DecodedTransport,
    entry_name: &str,
    profile: CompressionProfile,
) -> Result<LegacyImportResult> {
    let (path, components) = logical_path(entry_name)?;
    let mut synthesized = Vec::new();
    let mut entries = Vec::new();
    for depth in 1..components.len() {
        let ancestor = LogicalPath::from_utf8(&components[..depth])?;
        synthesized.push(ancestor.clone());
        entries.push(Entry::new(
            ancestor,
            EntryData::Directory,
            MetadataSet::default(),
            EntryIdentity::default(),
        ));
    }
    let digest = sha256_exact(&transport.decoded);
    entries.push(Entry::new(
        path,
        EntryData::File {
            content: ContentRef::Internal(digest),
        },
        MetadataSet::new(vec![MetadataItem::executable(false)])?,
        EntryIdentity::default(),
    ));
    let mut resolutions = synthesized
        .iter()
        .map(|path| crate::eam::ConversionResolution {
            conflict_class: "omission".to_owned(),
            semantic_field: format!("directory:{path}"),
            authorities: Box::from(["caller entry-name".to_owned()]),
            observed_values: Box::from(["directory absent from stream".to_owned()]),
            action: "synthesized explicit ancestor required by EAM".to_owned(),
        })
        .collect::<Vec<_>>();
    resolutions.push(crate::eam::ConversionResolution {
        conflict_class: "refinement".to_owned(),
        semantic_field: "layer.transport-decoded-child".to_owned(),
        authorities: Box::from([
            transport.format.as_str().to_owned(),
            "caller entry-name".to_owned(),
        ]),
        observed_values: Box::from([
            transport.observation.source_digest.to_string(),
            transport.decoded_digest.to_string(),
        ]),
        action: format!(
            "verified {} transport ({} members) and projected decoded bytes as {entry_name}",
            transport.format.as_str(),
            transport.member_count
        ),
    });
    let source_format = transport.format.as_str().to_owned();
    let provenance = ConversionProvenance {
        source_format: source_format.clone(),
        adapter_id: format!("{source_format}-stream-strict/v1"),
        source_digest: transport.observation.source_digest,
        import_mode: "strict".to_owned(),
        source_entry_count: 1,
        observation_count: transport.observation.observation_count(),
        omission_count: u64::try_from(synthesized.len()).unwrap_or(u64::MAX),
        refinement_count: 1,
        divergence_count: 0,
        irreconcilable_count: 0,
        resolutions: resolutions.into_boxed_slice(),
        synthesized_ancestors: synthesized.clone().into_boxed_slice(),
        unsupported_metadata: Box::from([format!("{source_format}.wrapper-metadata")]),
        outcome: "success".to_owned(),
    };
    let fidelity = FidelityReport {
        captured: Box::from(["legacy.conversion-provenance".to_owned()]),
        unavailable: Box::from([FidelityIssue {
            class: format!("{source_format}.wrapper-metadata"),
            reason: "transport metadata remains auxiliary evidence and is not native file metadata"
                .to_owned(),
            entry_scope: None,
        }]),
        degraded: Box::default(),
        platform: format!("legacy:{source_format}"),
        filesystem: Box::default(),
    };
    let observation = transport.observation;
    let member_count = transport.member_count;
    let decoded_digest = transport.decoded_digest;
    let archive = plan_observed_archive(
        entries,
        vec![transport.decoded],
        fidelity,
        provenance,
        None,
        profile,
    )?;
    Ok(LegacyImportResult {
        archive,
        report: LegacyConversionReport {
            observation,
            synthesized_ancestors: synthesized.into_boxed_slice(),
            layers: Box::from([source_format]),
            wrapper_members: member_count,
            decoded_child_digest: Some(decoded_digest),
            projection: format!("single-file:{entry_name}"),
        },
    })
}

fn logical_path(value: &str) -> Result<(LogicalPath, Vec<String>)> {
    if value.is_empty()
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value.as_bytes().contains(&0)
        || value.as_bytes().get(1).is_some_and(|byte| *byte == b':')
    {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::TarUnsafePath,
            format!("unsafe standalone entry name '{value}'"),
        ));
    }
    let components = value.split('/').map(str::to_owned).collect::<Vec<_>>();
    if components
        .iter()
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::TarUnsafePath,
            format!("unsafe standalone entry name '{value}'"),
        ));
    }
    LogicalPath::from_utf8(&components)
        .map(|path| (path, components))
        .map_err(|error| {
            Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::TarUnsafePath,
                error.detail(),
            )
        })
}

fn format_mismatch(expected: LegacySourceFormat, actual: LegacySourceFormat) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::LegacyFormatUnsupported,
        format!(
            "--from {} does not match detected {} framing",
            expected.as_str(),
            actual.as_str()
        ),
    )
}

fn usage(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::CommandUsage,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use crate::ecf::{StreamWriteOptions, WriteOptions, encode, encode_stream};
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use lzma_rust2::{XzOptions, XzWriter};

    use super::*;

    fn tar(value: &[u8]) -> Vec<u8> {
        let mut header = [0_u8; 512];
        header[..8].copy_from_slice(b"file.bin");
        write_octal(&mut header[100..108], 0o755);
        write_octal(&mut header[108..116], 0);
        write_octal(&mut header[116..124], 0);
        write_octal(&mut header[124..136], u64::try_from(value.len()).unwrap());
        write_octal(&mut header[136..148], 1_700_000_000);
        header[148..156].fill(b' ');
        header[156] = b'0';
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        let checksum = header.iter().map(|byte| u64::from(*byte)).sum::<u64>();
        header[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());
        let mut output = header.to_vec();
        output.extend_from_slice(value);
        output.resize(output.len().div_ceil(512) * 512 + 1024, 0);
        output
    }

    fn write_octal(field: &mut [u8], value: u64) {
        field.fill(0);
        let text = format!("{:0width$o}", value, width = field.len() - 1);
        field[..text.len()].copy_from_slice(text.as_bytes());
    }

    fn gzip(value: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(value).unwrap();
        encoder.finish().unwrap()
    }

    fn xz(value: &[u8]) -> Vec<u8> {
        let mut encoder = XzWriter::new(Vec::new(), XzOptions::with_preset(1)).unwrap();
        encoder.write_all(value).unwrap();
        encoder.finish().unwrap()
    }

    fn bzip2(value: &[u8]) -> Vec<u8> {
        let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::fast());
        encoder.write_all(value).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn every_transport_composes_with_the_one_tar_observer() {
        let child = tar(b"exact bytes");
        let sources = [
            (LegacySourceFormat::TarGzip, gzip(&child)),
            (
                LegacySourceFormat::TarZstandard,
                zstd::stream::encode_all(child.as_slice(), 1).unwrap(),
            ),
            (LegacySourceFormat::TarXz, xz(&child)),
            (LegacySourceFormat::TarBzip2, bzip2(&child)),
        ];
        let bare = import_strict(
            &child,
            Some(LegacySourceFormat::Tar),
            None,
            LegacyImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        for (format, source) in sources {
            let imported = import_strict(
                &source,
                Some(format),
                None,
                LegacyImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap();
            assert_eq!(imported.report.layers.len(), 2);
            assert_eq!(imported.archive.descriptor.lai, bare.archive.descriptor.lai);
            assert_eq!(imported.archive.descriptor.pcr, bare.archive.descriptor.pcr);
            assert_ne!(imported.archive.descriptor.aux, bare.archive.descriptor.aux);
            let indexed = encode(&imported.archive, WriteOptions::default()).unwrap();
            let mut stream = Vec::new();
            let streamed = encode_stream(
                &imported.archive,
                StreamWriteOptions::default(),
                &mut stream,
            )
            .unwrap();
            assert_eq!(indexed.identities.lai, streamed.identities.lai);
            assert_eq!(indexed.identities.pcr, streamed.identities.pcr);
            assert_eq!(indexed.identities.aux, streamed.identities.aux);
            assert_ne!(indexed.identities.pci, streamed.identities.pci);
        }
    }

    #[test]
    fn standalone_stream_requires_and_validates_an_explicit_entry_name() {
        let source = gzip(b"standalone");
        assert_eq!(
            import_strict(
                &source,
                Some(LegacySourceFormat::Gzip),
                None,
                LegacyImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap_err()
            .code(),
            ReasonCode::CommandUsage
        );
        let imported = import_strict(
            &source,
            Some(LegacySourceFormat::Gzip),
            Some("safe/data.bin"),
            LegacyImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        assert_eq!(imported.report.projection, "single-file:safe/data.bin");
        assert_eq!(imported.report.synthesized_ancestors.len(), 1);
        assert_eq!(
            import_strict(
                &source,
                Some(LegacySourceFormat::Gzip),
                Some("../escape"),
                LegacyImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap_err()
            .code(),
            ReasonCode::TarUnsafePath
        );
    }

    #[test]
    fn explicit_format_must_match_actual_framing() {
        let source = gzip(b"not tar");
        assert_eq!(
            import_strict(
                &source,
                Some(LegacySourceFormat::Xz),
                Some("file"),
                LegacyImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap_err()
            .code(),
            ReasonCode::LegacyFormatUnsupported
        );
    }
}
