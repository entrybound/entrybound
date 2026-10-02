//! Canonical crypto-v1 transcripts and private-object grammars.

use std::cmp::Ordering;

use super::working::{CryptoWorkContext, WorkBytes, WorkVec};
use crate::canonical::{RecordBuilder, decode_record_view as decode_record};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};

pub(crate) const FORMAT_NAMESPACE: &[u8] = b"ecf/bootstrap-v1";
pub(crate) const CRYPTO_VERSION: u16 = 1;
pub(crate) const PAYLOAD_SUITE_V1: u16 = 1;
pub(crate) const XWING_METHOD: &[u8] = b"xwing-mlkem768-x25519-sha3-256/draft-10";
pub(crate) const PASSWORD_METHOD_LEN: usize = 36;

pub(crate) const RECORD_CRYPTO_ENVELOPE: u16 = 20;
pub(crate) const RECORD_RECIPIENT_STANZA: u16 = 21;
pub(crate) const RECORD_RECIPIENT_DIRECTORY: u16 = 22;
pub(crate) const RECORD_PRIVATE_FRAGMENT: u16 = 23;
pub(crate) const RECORD_SEGMENT_END: u16 = 24;
pub(crate) const RECORD_ARCHIVE_FINAL: u16 = 25;
pub(crate) const RECORD_SIGNATURE: u16 = 26;
pub(crate) const RECORD_ENCRYPTED_INDEX: u16 = 27;

pub(crate) const PRIVATE_OBJECT_RECORD: u16 = 1;
pub(crate) const PRIVATE_OBJECT_CHUNK: u16 = 2;
pub(crate) const PRIVATE_OBJECT_SEQUENCE: u16 = 3;

pub(crate) const COLLECTION_MANIFEST: u16 = 1;
pub(crate) const COLLECTION_TRANSFORM_PLANS: u16 = 2;
pub(crate) const COLLECTION_DICTIONARIES: u16 = 3;
pub(crate) const COLLECTION_CHUNK_GROUPS: u16 = 4;
pub(crate) const COLLECTION_RECONSTRUCTION_DATA: u16 = 5;
pub(crate) const COLLECTION_RECONSTRUCTION_REGIONS: u16 = 6;
pub(crate) const COLLECTION_SIGNATURES: u16 = 7;
pub(crate) const COLLECTION_RECIPIENT_DIRECTORY: u16 = 8;
pub(crate) const COLLECTION_INDEX: u16 = 9;

const MAX_T1_FIELDS: usize = u16::MAX as usize;
const MAX_PRIVATE_OBJECT: usize = 1 << 30;
const MAX_SEQUENCE_ITEMS: usize = 1_000_000;
const MAX_SEQUENCE_BYTES: usize = 1 << 30;
const MAX_SEQUENCE_ITEM_BYTES: usize = 64 << 20;

/// Encodes the exact closed T1 transcript grammar.
pub(crate) fn t1(label: &str, fields: &[&[u8]]) -> Result<Vec<u8>> {
    if !label.is_ascii() || label.is_empty() || label.len() > usize::from(u16::MAX) {
        return Err(private_invalid("T1 label is not canonical ASCII"));
    }
    if fields.len() > MAX_T1_FIELDS {
        return Err(private_invalid("T1 field count exceeds u16"));
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(label.len() as u16).to_be_bytes());
    bytes.extend_from_slice(label.as_bytes());
    bytes.extend_from_slice(&(fields.len() as u16).to_be_bytes());
    for (index, value) in fields.iter().enumerate() {
        bytes.extend_from_slice(&((index + 1) as u16).to_be_bytes());
        bytes.extend_from_slice(
            &u64::try_from(value.len())
                .map_err(|_| private_invalid("T1 field exceeds u64"))?
                .to_be_bytes(),
        );
        bytes.extend_from_slice(value);
    }
    Ok(bytes)
}

/// Strictly decodes a closed T1 transcript and checks its label/field count.
#[allow(dead_code)]
pub(crate) fn decode_t1<'a>(
    input: &'a [u8],
    expected_label: &str,
    expected_fields: usize,
) -> Result<Vec<&'a [u8]>> {
    let mut cursor = 0usize;
    let label_len = take_u16(input, &mut cursor)? as usize;
    let label = take(input, &mut cursor, label_len)?;
    if label != expected_label.as_bytes() {
        return Err(private_invalid(
            "T1 label does not match its closed transcript",
        ));
    }
    let count = take_u16(input, &mut cursor)? as usize;
    if count != expected_fields {
        return Err(private_invalid("T1 field count is not canonical"));
    }
    // A field may be empty, but still needs its tag and length (ten bytes).
    if count > input.len().saturating_sub(cursor) / 10 {
        return Err(private_invalid("T1 field count exceeds its encoded extent"));
    }
    let mut fields = Vec::new();
    fields
        .try_reserve_exact(count)
        .map_err(|_| super::resource_refused("cannot reserve T1 field list"))?;
    for expected in 1..=count {
        if take_u16(input, &mut cursor)? as usize != expected {
            return Err(private_invalid("T1 tags must be contiguous and increasing"));
        }
        let len = to_usize(take_u64(input, &mut cursor)?)?;
        fields.push(take(input, &mut cursor, len)?);
    }
    if cursor != input.len() {
        return Err(private_invalid("T1 transcript has trailing bytes"));
    }
    Ok(fields)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecipientStanza {
    pub stanza_type: u16,
    pub protection_class: u8,
    pub stanza_id: [u8; 16],
    pub recipient_hint: [u8; 16],
    pub method_parameters: Vec<u8>,
    pub encapsulation: Vec<u8>,
    pub wrap_nonce: [u8; 12],
    pub wrapped_afk: [u8; 48],
}

impl RecipientStanza {
    pub(crate) fn fields(&self) -> StanzaFields<'_> {
        StanzaFields {
            stanza_type: self.stanza_type,
            protection_class: self.protection_class,
            stanza_id: self.stanza_id,
            recipient_hint: self.recipient_hint,
            method_parameters: &self.method_parameters,
            encapsulation: &self.encapsulation,
            wrap_nonce: self.wrap_nonce,
            wrapped_afk: self.wrapped_afk,
        }
    }

    pub(crate) fn encode(&self) -> Result<Vec<u8>> {
        let mut record = RecordBuilder::new(RECORD_RECIPIENT_STANZA);
        record
            .u16(1, 1)?
            .u16(2, self.stanza_type)?
            .u8(3, self.protection_class)?
            .bytes(4, &self.stanza_id)?
            .bytes(5, &self.recipient_hint)?
            .bytes(6, &self.method_parameters)?
            .bytes(7, &self.encapsulation)?
            .bytes(8, &self.wrap_nonce)?
            .bytes(9, &self.wrapped_afk)?;
        record.finish()
    }

    #[cfg(test)]
    pub(crate) fn decode(input: &[u8]) -> Result<Self> {
        let (record, consumed) = decode_record(input)?;
        if consumed != input.len() || record.kind != RECORD_RECIPIENT_STANZA {
            return Err(stanza_invalid(
                "RecipientStanza must be one complete type-21 record",
            ));
        }
        record.expect_tags(&[1, 2, 3, 4, 5, 6, 7, 8, 9], &[])?;
        if record.field(1)?.as_u16()? != 1 {
            return Err(stanza_invalid("unsupported RecipientStanza version"));
        }
        let value = Self {
            stanza_type: record.field(2)?.as_u16()?,
            protection_class: record.field(3)?.as_u8()?,
            stanza_id: exact(record.field(4)?.as_bytes()?)?,
            recipient_hint: exact(record.field(5)?.as_bytes()?)?,
            method_parameters: record.field(6)?.as_bytes()?.to_vec(),
            encapsulation: record.field(7)?.as_bytes()?.to_vec(),
            wrap_nonce: exact(record.field(8)?.as_bytes()?)?,
            wrapped_afk: exact(record.field(9)?.as_bytes()?)?,
        };
        if value.recipient_hint != [0; 16] || value.encode()? != input {
            return Err(stanza_invalid("RecipientStanza is not canonical"));
        }
        Ok(value)
    }

    pub(crate) fn sort_key(&self) -> Result<([u8; 32], Vec<u8>)> {
        use sha2::{Digest as _, Sha256};
        let exact = self.encode()?;
        let transcript = t1("entrybound/recipient-stanza-sort/v1", &[&exact])?;
        Ok((Sha256::digest(transcript).into(), exact))
    }
}

/// Field-only adapter shared by borrowed readers and the existing owned writer.
/// It does not claim that an owned writer value has validated exact wire bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StanzaFields<'a> {
    pub stanza_type: u16,
    pub protection_class: u8,
    pub stanza_id: [u8; 16],
    pub recipient_hint: [u8; 16],
    pub method_parameters: &'a [u8],
    pub encapsulation: &'a [u8],
    pub wrap_nonce: [u8; 12],
    pub wrapped_afk: [u8; 48],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StanzaRef<'a> {
    exact: &'a [u8],
    fields: StanzaFields<'a>,
}

impl<'a> StanzaRef<'a> {
    pub(crate) fn decode(input: &'a [u8]) -> Result<Self> {
        let (record, consumed) = decode_record(input)?;
        if consumed != input.len() || record.kind != RECORD_RECIPIENT_STANZA {
            return Err(stanza_invalid(
                "RecipientStanza must be one complete type-21 record",
            ));
        }
        record.expect_tags(&[1, 2, 3, 4, 5, 6, 7, 8, 9], &[])?;
        if record.field(1)?.as_u16()? != 1 {
            return Err(stanza_invalid("unsupported RecipientStanza version"));
        }
        let fields = StanzaFields {
            stanza_type: record.field(2)?.as_u16()?,
            protection_class: record.field(3)?.as_u8()?,
            stanza_id: exact(record.field(4)?.as_bytes()?)?,
            recipient_hint: exact(record.field(5)?.as_bytes()?)?,
            method_parameters: record.field(6)?.as_bytes()?,
            encapsulation: record.field(7)?.as_bytes()?,
            wrap_nonce: exact(record.field(8)?.as_bytes()?)?,
            wrapped_afk: exact(record.field(9)?.as_bytes()?)?,
        };
        if fields.recipient_hint != [0; 16] {
            return Err(stanza_invalid("RecipientStanza is not canonical"));
        }
        Ok(Self {
            exact: input,
            fields,
        })
    }

    pub(crate) fn fields(self) -> StanzaFields<'a> {
        self.fields
    }
    pub(crate) fn exact_bytes(self) -> &'a [u8] {
        self.exact
    }

    fn sort_digest(self) -> Result<[u8; 32]> {
        hash_t1_single("entrybound/recipient-stanza-sort/v1", self.exact)
    }
}

#[derive(Clone, Copy, Debug)]
struct StanzaSpan {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StanzaSequenceRef<'a> {
    exact: &'a [u8],
    count: usize,
    spans: Option<&'a [StanzaSpan]>,
}

impl<'a> StanzaSequenceRef<'a> {
    pub(crate) fn exact_bytes(self) -> &'a [u8] {
        self.exact
    }
    pub(crate) fn len(self) -> usize {
        self.count
    }
    pub(crate) fn is_empty(self) -> bool {
        self.count == 0
    }
    pub(crate) fn iter(self) -> StanzaIter<'a> {
        StanzaIter {
            sequence: self,
            index: 0,
            cursor: 8,
        }
    }
}

pub(crate) struct StanzaIter<'a> {
    sequence: StanzaSequenceRef<'a>,
    index: usize,
    cursor: usize,
}

impl<'a> Iterator for StanzaIter<'a> {
    type Item = StanzaRef<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.sequence.count {
            return None;
        }
        let bytes = if let Some(spans) = self.sequence.spans {
            let span = spans[self.index];
            &self.sequence.exact[span.start..span.end]
        } else {
            let length = take_u64(self.sequence.exact, &mut self.cursor)
                .expect("validated stanza length") as usize;
            take(self.sequence.exact, &mut self.cursor, length).expect("validated stanza extent")
        };
        self.index += 1;
        Some(StanzaRef::decode(bytes).expect("immutable validated stanza"))
    }
}

enum StanzaAdmission {
    Count(usize),
    Item(StanzaSpan),
}

fn scan_stanzas<'a>(
    input: &'a [u8],
    limits: Option<(u32, u64)>,
    mut admit: impl FnMut(StanzaAdmission) -> Result<()>,
) -> Result<StanzaSequenceRef<'a>> {
    if input.len() > 16 << 20 {
        return Err(stanza_invalid("recipient sequence exceeds 16 MiB"));
    }
    let mut cursor = 0;
    let count = to_usize(take_u64(input, &mut cursor)?)?;
    if count == 0 || count > 1_024 {
        return Err(stanza_invalid(
            "recipient stanza count is outside v1 bounds",
        ));
    }
    if count > input.len().saturating_sub(cursor) / 9 {
        return Err(stanza_invalid(
            "recipient stanza count exceeds its encoded extent",
        ));
    }
    if limits.is_some_and(|(maximum, _)| count as u64 > u64::from(maximum)) {
        return Err(super::resource_refused(
            "recipient stanza count exceeds caller policy",
        ));
    }
    // Managed owners reserve here, before any item validation, as the old list did.
    admit(StanzaAdmission::Count(count))?;
    for _ in 0..count {
        let length = to_usize(take_u64(input, &mut cursor)?)?;
        if length == 0 || length > 65_536 {
            return Err(stanza_invalid(
                "recipient stanza length is outside v1 bounds",
            ));
        }
        let start = cursor;
        let bytes = take(input, &mut cursor, length)?;
        if limits.is_some_and(|(_, maximum)| length as u64 > maximum) {
            return Err(super::resource_refused(
                "recipient stanza exceeds caller policy",
            ));
        }
        StanzaRef::decode(bytes)?;
        admit(StanzaAdmission::Item(StanzaSpan { start, end: cursor }))?;
    }
    if cursor != input.len() {
        return Err(stanza_invalid("recipient stanza sequence is not canonical"));
    }
    let sequence = StanzaSequenceRef {
        exact: input,
        count,
        spans: None,
    };
    let mut previous: Option<([u8; 32], &[u8])> = None;
    for stanza in sequence.iter() {
        let key = (stanza.sort_digest()?, stanza.exact_bytes());
        if previous.is_some_and(|old| compare_stanza_keys(old, key) != Ordering::Less) {
            return Err(stanza_invalid(
                "recipient stanzas are not uniquely canonically ordered",
            ));
        }
        previous = Some(key);
    }
    Ok(sequence)
}

fn compare_stanza_keys(left: ([u8; 32], &[u8]), right: ([u8; 32], &[u8])) -> Ordering {
    left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1))
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct EnvelopeRef<'a> {
    exact: &'a [u8],
    pub archive_id: [u8; 32],
    pub commitment: [u8; 32],
    pub protection_policy: u8,
    pub padding_mode: u8,
    pub boundary_mode: u8,
    pub stanzas: StanzaSequenceRef<'a>,
    pub envelope_mac: [u8; 32],
}

impl<'a> EnvelopeRef<'a> {
    pub(crate) fn exact_bytes(self) -> &'a [u8] {
        self.exact
    }
    pub(crate) fn decode_with_policy(input: &'a [u8], policy: super::CryptoPolicy) -> Result<Self> {
        decode_envelope_ref(input, policy, |_| Ok(()))
    }
}

fn decode_envelope_ref<'a>(
    input: &'a [u8],
    policy: super::CryptoPolicy,
    admit: impl FnMut(StanzaAdmission) -> Result<()>,
) -> Result<EnvelopeRef<'a>> {
    if u64::try_from(input.len()).unwrap_or(u64::MAX) > policy.max_envelope_bytes {
        return Err(super::resource_refused(
            "CryptoEnvelope exceeds caller policy",
        ));
    }
    let (record, consumed) = decode_record(input)?;
    if consumed != input.len() || record.kind != RECORD_CRYPTO_ENVELOPE {
        return Err(stanza_invalid(
            "CryptoEnvelope must be one complete type-20 record",
        ));
    }
    record.expect_tags(&[1, 2, 3, 4, 5, 6, 7, 8, 9], &[])?;
    if record.field(1)?.as_u16()? != CRYPTO_VERSION
        || record.field(2)?.as_u16()? != PAYLOAD_SUITE_V1
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::CryptoSuiteUnsupported,
            "unsupported encrypted archive suite",
        ));
    }
    Ok(EnvelopeRef {
        exact: input,
        archive_id: exact(record.field(3)?.as_bytes()?)?,
        commitment: exact(record.field(4)?.as_bytes()?)?,
        protection_policy: record.field(5)?.as_u8()?,
        padding_mode: record.field(6)?.as_u8()?,
        boundary_mode: record.field(7)?.as_u8()?,
        stanzas: scan_stanzas(
            record.field(8)?.as_bytes()?,
            Some((policy.max_stanzas, policy.max_stanza_bytes)),
            admit,
        )?,
        envelope_mac: exact(record.field(9)?.as_bytes()?)?,
    })
}

/// One charged raw envelope plus its charged, nonsecret stanza span table.
pub(crate) struct WorkEnvelope {
    raw: WorkBytes,
    spans: WorkVec<StanzaSpan>,
    sequence_start: usize,
    sequence_end: usize,
    archive_id: [u8; 32],
    commitment: [u8; 32],
    protection_policy: u8,
    padding_mode: u8,
    boundary_mode: u8,
    envelope_mac: [u8; 32],
}

impl WorkEnvelope {
    pub(crate) fn decode_with_context(raw: WorkBytes, work: &CryptoWorkContext) -> Result<Self> {
        if !raw.belongs_to(work.budget()) {
            return Err(super::resource_refused(
                "CryptoEnvelope backing belongs to another working budget",
            ));
        }
        let mut spans = WorkVec::new(work.budget());
        let view =
            decode_envelope_ref(raw.as_slice(), work.policy(), |admission| match admission {
                StanzaAdmission::Count(count) => spans.reserve(count),
                StanzaAdmission::Item(span) => spans.push(span),
            })?;
        let sequence_start = (view.stanzas.exact.as_ptr() as usize)
            .checked_sub(raw.as_slice().as_ptr() as usize)
            .expect("sequence is a field of the raw envelope");
        let sequence_end = sequence_start
            .checked_add(view.stanzas.exact.len())
            .expect("validated envelope extent");
        let result = Self {
            archive_id: view.archive_id,
            commitment: view.commitment,
            protection_policy: view.protection_policy,
            padding_mode: view.padding_mode,
            boundary_mode: view.boundary_mode,
            envelope_mac: view.envelope_mac,
            sequence_start,
            sequence_end,
            spans,
            raw,
        };
        Ok(result)
    }

    pub(crate) fn view(&self) -> EnvelopeRef<'_> {
        EnvelopeRef {
            exact: self.raw.as_slice(),
            archive_id: self.archive_id,
            commitment: self.commitment,
            protection_policy: self.protection_policy,
            padding_mode: self.padding_mode,
            boundary_mode: self.boundary_mode,
            envelope_mac: self.envelope_mac,
            stanzas: StanzaSequenceRef {
                exact: &self.raw[self.sequence_start..self.sequence_end],
                count: self.spans.as_slice().len(),
                spans: Some(self.spans.as_slice()),
            },
        }
    }
}

impl std::fmt::Debug for WorkEnvelope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkEnvelope")
            .field("raw_length", &self.raw.len())
            .field("stanza_count", &self.spans.as_slice().len())
            .finish_non_exhaustive()
    }
}

pub(crate) fn encode_stanza_sequence(stanzas: &[RecipientStanza]) -> Result<Vec<u8>> {
    if stanzas.is_empty() || stanzas.len() > 1_024 {
        return Err(stanza_invalid(
            "recipient stanza count is outside v1 bounds",
        ));
    }
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&(stanzas.len() as u64).to_be_bytes());
    let mut previous: Option<([u8; 32], Vec<u8>)> = None;
    for stanza in stanzas {
        let key = stanza.sort_key()?;
        if previous
            .as_ref()
            .is_some_and(|prior| prior.cmp(&key) != Ordering::Less)
        {
            return Err(stanza_invalid(
                "recipient stanzas are not uniquely canonically ordered",
            ));
        }
        if key.1.len() > 65_536 {
            return Err(stanza_invalid("recipient stanza exceeds 64 KiB"));
        }
        encoded.extend_from_slice(&(key.1.len() as u64).to_be_bytes());
        encoded.extend_from_slice(&key.1);
        previous = Some(key);
    }
    if encoded.len() > 16 << 20 {
        return Err(stanza_invalid("recipient sequence exceeds 16 MiB"));
    }
    Ok(encoded)
}

#[cfg(test)]
pub(crate) fn decode_stanza_sequence(input: &[u8]) -> Result<Vec<RecipientStanza>> {
    decode_stanza_sequence_with_limits(input, None)
}

#[cfg(test)]
fn decode_stanza_sequence_with_limits(
    input: &[u8],
    caller_limits: Option<(u32, u64)>,
) -> Result<Vec<RecipientStanza>> {
    if input.len() > 16 << 20 {
        return Err(stanza_invalid("recipient sequence exceeds 16 MiB"));
    }
    let mut cursor = 0usize;
    let count = to_usize(take_u64(input, &mut cursor)?)?;
    if count == 0 || count > 1_024 {
        return Err(stanza_invalid(
            "recipient stanza count is outside v1 bounds",
        ));
    }
    // Every stanza occupies at least an eight-byte length and one data byte.
    // Check the physical encoding before reserving from an untrusted count.
    if count > input.len().saturating_sub(cursor) / 9 {
        return Err(stanza_invalid(
            "recipient stanza count exceeds its encoded extent",
        ));
    }
    if caller_limits.is_some_and(|(max_stanzas, _)| count as u64 > u64::from(max_stanzas)) {
        return Err(super::resource_refused(
            "recipient stanza count exceeds caller policy",
        ));
    }
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| super::resource_refused("cannot reserve recipient stanza list"))?;
    for _ in 0..count {
        let len = to_usize(take_u64(input, &mut cursor)?)?;
        if len == 0 || len > 65_536 {
            return Err(stanza_invalid(
                "recipient stanza length is outside v1 bounds",
            ));
        }
        let stanza_bytes = take(input, &mut cursor, len)?;
        if caller_limits.is_some_and(|(_, max_stanza_bytes)| len as u64 > max_stanza_bytes) {
            return Err(super::resource_refused(
                "recipient stanza exceeds caller policy",
            ));
        }
        values.push(RecipientStanza::decode(stanza_bytes)?);
    }
    if cursor != input.len() || encode_stanza_sequence(&values)? != input {
        return Err(stanza_invalid("recipient stanza sequence is not canonical"));
    }
    Ok(values)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CryptoEnvelope {
    pub archive_id: [u8; 32],
    pub commitment: [u8; 32],
    pub protection_policy: u8,
    pub padding_mode: u8,
    pub boundary_mode: u8,
    pub stanzas: Vec<RecipientStanza>,
    pub envelope_mac: [u8; 32],
}

impl CryptoEnvelope {
    pub(crate) fn encode(&self) -> Result<Vec<u8>> {
        let sequence = encode_stanza_sequence(&self.stanzas)?;
        let mut record = RecordBuilder::new(RECORD_CRYPTO_ENVELOPE);
        record
            .u16(1, CRYPTO_VERSION)?
            .u16(2, PAYLOAD_SUITE_V1)?
            .bytes(3, &self.archive_id)?
            .bytes(4, &self.commitment)?
            .u8(5, self.protection_policy)?
            .u8(6, self.padding_mode)?
            .u8(7, self.boundary_mode)?
            .bytes(8, &sequence)?
            .bytes(9, &self.envelope_mac)?;
        record.finish()
    }

    #[cfg(test)]
    pub(crate) fn decode(input: &[u8]) -> Result<Self> {
        Self::decode_inner(input, None)
    }

    #[cfg(test)]
    pub(crate) fn decode_with_policy(input: &[u8], policy: super::CryptoPolicy) -> Result<Self> {
        if u64::try_from(input.len()).unwrap_or(u64::MAX) > policy.max_envelope_bytes {
            return Err(super::resource_refused(
                "CryptoEnvelope exceeds caller policy",
            ));
        }
        Self::decode_inner(input, Some((policy.max_stanzas, policy.max_stanza_bytes)))
    }

    #[cfg(test)]
    fn decode_inner(input: &[u8], caller_limits: Option<(u32, u64)>) -> Result<Self> {
        let (record, consumed) = decode_record(input)?;
        if consumed != input.len() || record.kind != RECORD_CRYPTO_ENVELOPE {
            return Err(stanza_invalid(
                "CryptoEnvelope must be one complete type-20 record",
            ));
        }
        record.expect_tags(&[1, 2, 3, 4, 5, 6, 7, 8, 9], &[])?;
        if record.field(1)?.as_u16()? != CRYPTO_VERSION
            || record.field(2)?.as_u16()? != PAYLOAD_SUITE_V1
        {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::CryptoSuiteUnsupported,
                "unsupported encrypted archive suite",
            ));
        }
        let value = Self {
            archive_id: exact(record.field(3)?.as_bytes()?)?,
            commitment: exact(record.field(4)?.as_bytes()?)?,
            protection_policy: record.field(5)?.as_u8()?,
            padding_mode: record.field(6)?.as_u8()?,
            boundary_mode: record.field(7)?.as_u8()?,
            stanzas: decode_stanza_sequence_with_limits(
                record.field(8)?.as_bytes()?,
                caller_limits,
            )?,
            envelope_mac: exact(record.field(9)?.as_bytes()?)?,
        };
        if value.encode()? != input {
            return Err(stanza_invalid("CryptoEnvelope is not canonical"));
        }
        Ok(value)
    }
}

pub(crate) fn private_object(kind: u16, payload: &[u8]) -> Result<Vec<u8>> {
    if !matches!(kind, 1..=3) || payload.len() + 12 > MAX_PRIVATE_OBJECT {
        return Err(private_invalid("private object kind or size is invalid"));
    }
    let mut out = b"EBPO".to_vec();
    out.extend_from_slice(&1_u16.to_be_bytes());
    out.extend_from_slice(&kind.to_be_bytes());
    out.extend_from_slice(&0_u32.to_be_bytes());
    out.extend_from_slice(payload);
    Ok(out)
}

pub(crate) fn decode_private_object(input: &[u8]) -> Result<(u16, &[u8])> {
    if input.len() < 12 || input.len() > MAX_PRIVATE_OBJECT || &input[..4] != b"EBPO" {
        return Err(private_invalid("private object header is malformed"));
    }
    if u16::from_be_bytes(exact(&input[4..6])?) != 1
        || u32::from_be_bytes(exact(&input[8..12])?) != 0
    {
        return Err(private_invalid(
            "private object version or flags are invalid",
        ));
    }
    let kind = u16::from_be_bytes(exact(&input[6..8])?);
    if !matches!(kind, 1..=3) {
        return Err(private_invalid("unknown private object kind"));
    }
    Ok((kind, &input[12..]))
}

pub(crate) fn sequence_container(kind: u16, items: &[Vec<u8>]) -> Result<Vec<u8>> {
    validate_collection_kind(kind)?;
    if items.len() > MAX_SEQUENCE_ITEMS {
        return Err(private_invalid("EBCS item count exceeds v1 limit"));
    }
    let mut out = b"EBCS".to_vec();
    out.extend_from_slice(&1_u16.to_be_bytes());
    out.extend_from_slice(&kind.to_be_bytes());
    out.extend_from_slice(&0_u32.to_be_bytes());
    out.extend_from_slice(&(items.len() as u64).to_be_bytes());
    for item in items {
        if item.is_empty() || item.len() > MAX_SEQUENCE_ITEM_BYTES {
            return Err(private_invalid("EBCS item length is outside v1 bounds"));
        }
        let (_, consumed) = decode_record(item)?;
        if consumed != item.len() {
            return Err(private_invalid(
                "EBCS item is not one complete canonical record",
            ));
        }
        out.extend_from_slice(&(item.len() as u64).to_be_bytes());
        out.extend_from_slice(item);
    }
    if out.len() > MAX_SEQUENCE_BYTES {
        return Err(private_invalid("EBCS extent exceeds 1 GiB"));
    }
    SequenceContainerRef::decode(&out)?;
    Ok(out)
}

pub(crate) fn decode_sequence_container(input: &[u8]) -> Result<(u16, Vec<Vec<u8>>)> {
    let mut items = Vec::new();
    let view = SequenceContainerRef::decode_with_admission(input, |admission| match admission {
        CollectionAdmission::Count(count) => items
            .try_reserve_exact(count)
            .map_err(|_| super::resource_refused("cannot reserve EBCS item list")),
        CollectionAdmission::Item(item) => {
            items.push(item.to_vec());
            Ok(())
        }
    })?;
    debug_assert_eq!(items.len(), view.len());
    Ok((view.kind(), items))
}

/// Complete framing and semantics are validated before exposing infallible iteration.
/// This is a borrowed view, not the deferred managed retained EBCS owner.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SequenceContainerRef<'a> {
    exact: &'a [u8],
    kind: u16,
    count: usize,
}

impl<'a> SequenceContainerRef<'a> {
    pub(crate) fn decode(input: &'a [u8]) -> Result<Self> {
        Self::decode_with_admission(input, |_| Ok(()))
    }
    pub(crate) fn kind(self) -> u16 {
        self.kind
    }
    pub(crate) fn len(self) -> usize {
        self.count
    }
    pub(crate) fn iter(self) -> CollectionIter<'a> {
        CollectionIter {
            exact: self.exact,
            remaining: self.count,
            cursor: 20,
        }
    }

    fn decode_with_admission(
        input: &'a [u8],
        mut admit: impl FnMut(CollectionAdmission<'a>) -> Result<()>,
    ) -> Result<Self> {
        let (kind, count) = collection_header(input)?;
        // The compatibility list still reserves at the old count-admission point.
        admit(CollectionAdmission::Count(count))?;
        let mut cursor = 20;
        for _ in 0..count {
            let length = to_usize(take_u64(input, &mut cursor)?)?;
            if length == 0 || length > MAX_SEQUENCE_ITEM_BYTES {
                return Err(private_invalid("EBCS item length is outside v1 bounds"));
            }
            let item = take(input, &mut cursor, length)?;
            // Preserve the compatibility owner's old copy-before-record-decode
            // boundary. Pure borrowed decoding has a no-op admission callback.
            admit(CollectionAdmission::Item(item))?;
            let (_, consumed) = decode_record(item)?;
            if consumed != item.len() {
                return Err(private_invalid("EBCS item has trailing bytes"));
            }
        }
        if cursor != input.len() {
            return Err(private_invalid("EBCS has trailing or truncated bytes"));
        }
        let view = Self {
            exact: input,
            kind,
            count,
        };
        validate_collection_view(view)?;
        Ok(view)
    }
}

enum CollectionAdmission<'a> {
    Count(usize),
    Item(&'a [u8]),
}

pub(crate) struct CollectionIter<'a> {
    exact: &'a [u8],
    remaining: usize,
    cursor: usize,
}
impl<'a> Iterator for CollectionIter<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let length =
            take_u64(self.exact, &mut self.cursor).expect("validated EBCS length") as usize;
        let item = take(self.exact, &mut self.cursor, length).expect("validated EBCS extent");
        self.remaining -= 1;
        Some(item)
    }
}

fn collection_header(input: &[u8]) -> Result<(u16, usize)> {
    if input.len() < 20 || input.len() > MAX_SEQUENCE_BYTES || &input[..4] != b"EBCS" {
        return Err(private_invalid("EBCS header or extent is invalid"));
    }
    if u16::from_be_bytes(exact(&input[4..6])?) != 1
        || u32::from_be_bytes(exact(&input[8..12])?) != 0
    {
        return Err(private_invalid("EBCS version or flags are invalid"));
    }
    let kind = u16::from_be_bytes(exact(&input[6..8])?);
    validate_collection_kind(kind)?;
    let count = to_usize(u64::from_be_bytes(exact(&input[12..20])?))?;
    if count > MAX_SEQUENCE_ITEMS {
        return Err(private_invalid("EBCS item count exceeds v1 limit"));
    }
    // A nonempty item needs an eight-byte length followed by at least one
    // byte. The fixed EBCS header was already checked above.
    if count > (input.len() - 20) / 9 {
        return Err(private_invalid(
            "EBCS item count exceeds its encoded extent",
        ));
    }
    Ok((kind, count))
}

fn validate_collection_kind(kind: u16) -> Result<()> {
    if !(1..=9).contains(&kind) {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::CryptoSuiteUnsupported,
            "unknown EBCS collection kind",
        ));
    }
    Ok(())
}

fn collection_types(kind: u16) -> &'static [u16] {
    match kind {
        COLLECTION_MANIFEST => &[3, 4],
        COLLECTION_TRANSFORM_PLANS => &[2],
        COLLECTION_DICTIONARIES => &[11],
        COLLECTION_CHUNK_GROUPS => &[12],
        COLLECTION_RECONSTRUCTION_DATA => &[14, 16],
        COLLECTION_RECONSTRUCTION_REGIONS => &[18, 19],
        COLLECTION_SIGNATURES => &[26],
        COLLECTION_RECIPIENT_DIRECTORY => &[22],
        COLLECTION_INDEX => &[27],
        _ => unreachable!("validated EBCS kind"),
    }
}

fn validate_collection_view(view: SequenceContainerRef<'_>) -> Result<()> {
    if matches!(
        view.kind,
        COLLECTION_TRANSFORM_PLANS | COLLECTION_SIGNATURES | COLLECTION_RECIPIENT_DIRECTORY
    ) && view.count == 0
    {
        return Err(private_invalid(
            "this EBCS collection kind may not be empty",
        ));
    }
    let mut previous = None;
    for item in view.iter() {
        let (record, _) = decode_record(item)?;
        if !collection_types(view.kind).contains(&record.kind) {
            return Err(private_invalid(
                "EBCS item type/order is invalid for its kind",
            ));
        }
        // Validate the entire current key before testing order against the prior key.
        let cursor = collection_cursor(view.kind, &record, item)?;
        if let Some(old) = previous {
            let (old_record, _) = decode_record(old)?;
            let old_cursor = collection_cursor(view.kind, &old_record, old)?;
            if old_cursor.cmp(cursor) != Ordering::Less {
                return Err(private_invalid("EBCS items are duplicate or out of order"));
            }
        }
        previous = Some(item);
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum KeyPart<'a> {
    Bytes(&'a [u8]),
    Fixed { bytes: [u8; 8], length: usize },
}

impl KeyPart<'_> {
    fn fixed(bytes: &[u8]) -> Self {
        let mut storage = [0; 8];
        storage[..bytes.len()].copy_from_slice(bytes);
        Self::Fixed {
            bytes: storage,
            length: bytes.len(),
        }
    }
    fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Bytes(bytes) => bytes,
            Self::Fixed { bytes, length } => &bytes[..*length],
        }
    }
}

/// Emits the exact allocated oracle key byte-for-byte, including across part boundaries.
/// Fixed stack state and a field-local path iterator replace the variable key Vec.
struct CollectionKeyCursor<'a> {
    parts: [KeyPart<'a>; 5],
    part_count: usize,
    part: usize,
    byte: usize,
    path: Option<crate::canonical::SequenceIter<'a>>,
}

impl<'a> CollectionKeyCursor<'a> {
    fn new(record_type: u16) -> Self {
        let mut result = Self {
            parts: [KeyPart::Bytes(&[]); 5],
            part_count: 1,
            part: 0,
            byte: 0,
            path: None,
        };
        result.parts[0] = KeyPart::fixed(&record_type.to_be_bytes());
        result
    }
    fn add(&mut self, part: KeyPart<'a>) {
        self.parts[self.part_count] = part;
        self.part_count += 1;
    }
}

impl Iterator for CollectionKeyCursor<'_> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        loop {
            if self.part < self.part_count {
                let bytes = self.parts[self.part].as_bytes();
                if let Some(value) = bytes.get(self.byte) {
                    self.byte += 1;
                    return Some(*value);
                }
                self.part += 1;
                self.byte = 0;
            } else {
                let component_bytes = self.path.as_mut()?.next()?;
                let (component, _) =
                    decode_record(component_bytes).expect("validated path component");
                let encoding = component
                    .field(1)
                    .expect("validated encoding field")
                    .as_u8()
                    .expect("validated encoding type");
                let bytes = component
                    .field(2)
                    .expect("validated component field")
                    .as_bytes()
                    .expect("validated component type");
                self.parts[0] = KeyPart::fixed(&[encoding]);
                self.parts[1] = KeyPart::fixed(&(bytes.len() as u64).to_be_bytes());
                self.parts[2] = KeyPart::Bytes(bytes);
                self.part_count = 3;
                self.part = 0;
                self.byte = 0;
            }
        }
    }
}

fn collection_cursor<'a>(
    kind: u16,
    record: &crate::canonical::RecordView<'a>,
    exact: &'a [u8],
) -> Result<CollectionKeyCursor<'a>> {
    let mut key = CollectionKeyCursor::new(record.kind);
    match kind {
        COLLECTION_MANIFEST if record.kind == 3 => {
            let path = record.field(1)?.as_sequence_view()?;
            for component_bytes in path.iter() {
                let (component, consumed) = decode_record(component_bytes)?;
                if consumed != component_bytes.len() || component.kind != 7 {
                    return Err(private_invalid("manifest path component is not canonical"));
                }
                component.expect_tags(&[1, 2], &[])?;
                component.field(1)?.as_u8()?;
                component.field(2)?.as_bytes()?;
            }
            key.path = Some(path.iter());
        }
        COLLECTION_MANIFEST => key.add(KeyPart::Bytes(record.field(1)?.as_bytes()?)),
        COLLECTION_TRANSFORM_PLANS => {
            key.add(KeyPart::fixed(&record.field(1)?.as_u64()?.to_be_bytes()))
        }
        COLLECTION_DICTIONARIES
        | COLLECTION_CHUNK_GROUPS
        | COLLECTION_RECONSTRUCTION_DATA
        | COLLECTION_RECIPIENT_DIRECTORY
        | COLLECTION_INDEX => key.add(KeyPart::Bytes(record.field(1)?.as_bytes()?)),
        COLLECTION_RECONSTRUCTION_REGIONS if record.kind == 18 => {
            key.add(KeyPart::Bytes(record.field(1)?.as_bytes()?))
        }
        COLLECTION_SIGNATURES => key.add(KeyPart::Bytes(exact)),
        COLLECTION_RECONSTRUCTION_REGIONS if record.kind == 19 => {
            key.add(KeyPart::fixed(&[record.field(1)?.as_u8()?]));
            key.add(KeyPart::Bytes(record.field(2)?.as_bytes()?));
            let transform = record.field(3)?.as_utf8()?.as_bytes();
            key.add(KeyPart::fixed(&(transform.len() as u64).to_be_bytes()));
            key.add(KeyPart::Bytes(transform));
        }
        _ => return Err(private_invalid("EBCS sort-key schema is unsupported")),
    }
    Ok(key)
}

#[cfg(test)]
fn validate_sequence_semantics(kind: u16, items: &[Vec<u8>]) -> Result<()> {
    if matches!(
        kind,
        COLLECTION_TRANSFORM_PLANS | COLLECTION_SIGNATURES | COLLECTION_RECIPIENT_DIRECTORY
    ) && items.is_empty()
    {
        return Err(private_invalid(
            "this EBCS collection kind may not be empty",
        ));
    }
    let allowed: &[u16] = match kind {
        COLLECTION_MANIFEST => &[3, 4],
        COLLECTION_TRANSFORM_PLANS => &[2],
        COLLECTION_DICTIONARIES => &[11],
        COLLECTION_CHUNK_GROUPS => &[12],
        COLLECTION_RECONSTRUCTION_DATA => &[14, 16],
        COLLECTION_RECONSTRUCTION_REGIONS => &[18, 19],
        COLLECTION_SIGNATURES => &[26],
        COLLECTION_RECIPIENT_DIRECTORY => &[22],
        COLLECTION_INDEX => &[27],
        _ => unreachable!(),
    };
    let mut previous = None::<Vec<u8>>;
    for item in items {
        let (record, _) = decode_record(item)?;
        if !allowed.contains(&record.kind) {
            return Err(private_invalid(
                "EBCS item type/order is invalid for its kind",
            ));
        }
        let key = collection_sort_key(kind, &record, item)?;
        if previous.as_ref().is_some_and(|old| old >= &key) {
            return Err(private_invalid("EBCS items are duplicate or out of order"));
        }
        previous = Some(key);
    }
    Ok(())
}

#[cfg(test)]
fn collection_sort_key(
    kind: u16,
    record: &crate::canonical::RecordView<'_>,
    exact: &[u8],
) -> Result<Vec<u8>> {
    let mut key = record.kind.to_be_bytes().to_vec();
    match kind {
        COLLECTION_MANIFEST if record.kind == 3 => {
            for component_bytes in record.field(1)?.as_sequence_view()?.iter() {
                let (component, consumed) = decode_record(component_bytes)?;
                if consumed != component_bytes.len() || component.kind != 7 {
                    return Err(private_invalid("manifest path component is not canonical"));
                }
                component.expect_tags(&[1, 2], &[])?;
                let encoding = component.field(1)?.as_u8()?;
                let bytes = component.field(2)?.as_bytes()?;
                key.push(encoding);
                key.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
                key.extend_from_slice(bytes);
            }
        }
        COLLECTION_MANIFEST => key.extend_from_slice(record.field(1)?.as_bytes()?),
        COLLECTION_TRANSFORM_PLANS => {
            key.extend_from_slice(&record.field(1)?.as_u64()?.to_be_bytes())
        }
        COLLECTION_DICTIONARIES
        | COLLECTION_CHUNK_GROUPS
        | COLLECTION_RECONSTRUCTION_DATA
        | COLLECTION_RECIPIENT_DIRECTORY
        | COLLECTION_INDEX => key.extend_from_slice(record.field(1)?.as_bytes()?),
        COLLECTION_RECONSTRUCTION_REGIONS if record.kind == 18 => {
            key.extend_from_slice(record.field(1)?.as_bytes()?);
        }
        COLLECTION_SIGNATURES => key.extend_from_slice(exact),
        _ if kind == COLLECTION_RECONSTRUCTION_DATA && record.kind == 16 => {
            key.extend_from_slice(record.field(1)?.as_bytes()?);
        }
        _ if kind == COLLECTION_RECONSTRUCTION_REGIONS && record.kind == 19 => {
            key.push(record.field(1)?.as_u8()?);
            key.extend_from_slice(record.field(2)?.as_bytes()?);
            let transform = record.field(3)?.as_utf8()?.as_bytes();
            key.extend_from_slice(&(transform.len() as u64).to_be_bytes());
            key.extend_from_slice(transform);
        }
        _ => return Err(private_invalid("EBCS sort-key schema is unsupported")),
    }
    Ok(key)
}

pub(crate) fn encode_private_fragment(
    object_id: &[u8; 32],
    total_len: u64,
    index: u32,
    count: u32,
    offset: u64,
    bytes: &[u8],
) -> Result<Vec<u8>> {
    if count == 0 || index >= count || bytes.is_empty() {
        return Err(private_invalid("private fragment cardinality is invalid"));
    }
    let mut record = RecordBuilder::new(RECORD_PRIVATE_FRAGMENT);
    record
        .bytes(1, object_id)?
        .u64(2, total_len)?
        .u32(3, index)?
        .u32(4, count)?
        .u64(5, offset)?
        .bytes(6, bytes)?;
    record.finish()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PrivateFragmentRef<'a> {
    pub object_id: [u8; 32],
    pub total_len: u64,
    pub index: u32,
    pub count: u32,
    pub offset: u64,
    pub bytes: &'a [u8],
}

pub(crate) fn decode_private_fragment_ref(input: &[u8]) -> Result<PrivateFragmentRef<'_>> {
    let (record, consumed) = decode_record(input)?;
    if consumed != input.len() || record.kind != RECORD_PRIVATE_FRAGMENT {
        return Err(private_invalid(
            "DATA plaintext is not one PrivateFragment record",
        ));
    }
    record.expect_tags(&[1, 2, 3, 4, 5, 6], &[])?;
    let value = PrivateFragmentRef {
        object_id: exact(record.field(1)?.as_bytes()?)?,
        total_len: record.field(2)?.as_u64()?,
        index: record.field(3)?.as_u32()?,
        count: record.field(4)?.as_u32()?,
        offset: record.field(5)?.as_u64()?,
        bytes: record.field(6)?.as_bytes()?,
    };
    if value.count == 0 || value.index >= value.count || value.bytes.is_empty() {
        return Err(private_invalid("PrivateFragment cardinality is invalid"));
    }
    Ok(value)
}

pub(crate) fn encrypted_object_id(bytes: &[u8]) -> Result<[u8; 32]> {
    hash_t1_single("entrybound/encrypted-object/v1", bytes)
}

/// Hashes the exact one-field T1 grammar without a transcript allocation.
fn hash_t1_single(label: &str, value: &[u8]) -> Result<[u8; 32]> {
    use sha2::{Digest as _, Sha256};
    if !label.is_ascii() || label.is_empty() || label.len() > usize::from(u16::MAX) {
        return Err(private_invalid("T1 label is not canonical ASCII"));
    }
    let length = u64::try_from(value.len()).map_err(|_| private_invalid("T1 field exceeds u64"))?;
    let mut hash = Sha256::new();
    hash.update((label.len() as u16).to_be_bytes());
    hash.update(label.as_bytes());
    hash.update(1_u16.to_be_bytes());
    hash.update(1_u16.to_be_bytes());
    hash.update(length.to_be_bytes());
    hash.update(value);
    Ok(hash.finalize().into())
}

pub(crate) fn record_kind(input: &[u8]) -> Result<u16> {
    let (record, consumed) = decode_record(input)?;
    if consumed != input.len() {
        return Err(private_invalid("expected exactly one canonical record"));
    }
    Ok(record.kind)
}

fn exact<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| private_invalid(format!("expected exactly {N} bytes")))
}

fn take<'a>(input: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8]> {
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| private_invalid("canonical length overflow"))?;
    let value = input
        .get(*cursor..end)
        .ok_or_else(|| private_invalid("canonical bytes are truncated"))?;
    *cursor = end;
    Ok(value)
}

fn take_u16(input: &[u8], cursor: &mut usize) -> Result<u16> {
    Ok(u16::from_be_bytes(exact(take(input, cursor, 2)?)?))
}

fn take_u64(input: &[u8], cursor: &mut usize) -> Result<u64> {
    Ok(u64::from_be_bytes(exact(take(input, cursor, 8)?)?))
}

fn to_usize(value: u64) -> Result<usize> {
    usize::try_from(value).map_err(|_| private_invalid("canonical length exceeds usize"))
}

pub(crate) fn private_invalid(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::CryptoPrivateObjectInvalid,
        detail,
    )
}

fn stanza_invalid(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::CryptoRecipientStanzaInvalid,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_envelope() -> CryptoEnvelope {
        CryptoEnvelope {
            archive_id: [4; 32],
            commitment: [5; 32],
            protection_policy: 2,
            padding_mode: 0,
            boundary_mode: 1,
            envelope_mac: [6; 32],
            stanzas: vec![RecipientStanza {
                stanza_type: 2,
                protection_class: 2,
                stanza_id: [1; 16],
                recipient_hint: [0; 16],
                method_parameters: vec![0; PASSWORD_METHOD_LEN],
                encapsulation: Vec::new(),
                wrap_nonce: [2; 12],
                wrapped_afk: [3; 48],
            }],
        }
    }

    fn raw_collection(kind: u16, items: &[Vec<u8>]) -> Vec<u8> {
        let mut out = b"EBCS".to_vec();
        out.extend_from_slice(&1_u16.to_be_bytes());
        out.extend_from_slice(&kind.to_be_bytes());
        out.extend_from_slice(&0_u32.to_be_bytes());
        out.extend_from_slice(&(items.len() as u64).to_be_bytes());
        for item in items {
            out.extend_from_slice(&(item.len() as u64).to_be_bytes());
            out.extend_from_slice(item);
        }
        out
    }

    #[test]
    fn envelope_views_match_owned_oracle_and_borrow_exact_variable_fields() {
        for stanza_type in [1, 2, 500] {
            let mut value = example_envelope();
            value.stanzas[0].stanza_type = stanza_type;
            let bytes = value.encode().unwrap();
            let policy = super::super::CryptoPolicy::default();
            let view = EnvelopeRef::decode_with_policy(&bytes, policy).unwrap();
            assert!(std::ptr::eq(view.exact_bytes().as_ptr(), bytes.as_ptr()));
            assert_eq!(view.archive_id, value.archive_id);
            assert_eq!(view.envelope_mac, value.envelope_mac);
            assert_eq!(view.stanzas.len(), 1);
            assert!(!view.stanzas.is_empty());
            let stanza = view.stanzas.iter().next().unwrap();
            assert_eq!(stanza.fields(), value.stanzas[0].fields());
            let (record, _) = decode_record(stanza.exact_bytes()).unwrap();
            let parameters = record.field(6).unwrap().as_bytes().unwrap();
            assert!(std::ptr::eq(
                stanza.fields().method_parameters.as_ptr(),
                parameters.as_ptr()
            ));
            assert_eq!(
                view.stanzas.exact_bytes(),
                encode_stanza_sequence(&value.stanzas).unwrap()
            );
            for length in 0..bytes.len() {
                let old = CryptoEnvelope::decode_with_policy(&bytes[..length], policy).unwrap_err();
                let new = EnvelopeRef::decode_with_policy(&bytes[..length], policy).unwrap_err();
                assert_eq!(
                    (new.class(), new.code()),
                    (old.class(), old.code()),
                    "prefix {length}"
                );
            }
            for limited in [
                super::super::CryptoPolicy {
                    max_stanzas: 0,
                    ..policy
                },
                super::super::CryptoPolicy {
                    max_stanza_bytes: 1,
                    ..policy
                },
            ] {
                assert_eq!(
                    EnvelopeRef::decode_with_policy(&bytes, limited).unwrap_err(),
                    CryptoEnvelope::decode_with_policy(&bytes, limited).unwrap_err()
                );
            }
        }
    }

    #[test]
    fn managed_envelope_retains_raw_and_spans_on_the_original_budget() {
        let bytes = example_envelope().encode().unwrap();
        let span_bytes = std::mem::size_of::<StanzaSpan>() as u64;
        let work = CryptoWorkContext::new(super::super::CryptoPolicy {
            max_working_memory_bytes: bytes.len() as u64 + span_bytes + 3,
            ..super::super::CryptoPolicy::default()
        });
        let retained = WorkBytes::zeroed(work.budget(), 3).unwrap();
        let mut raw = WorkBytes::zeroed(work.budget(), bytes.len()).unwrap();
        raw.copy_from_slice(&bytes);
        let original_pointer = raw.as_ptr();
        let owner = WorkEnvelope::decode_with_context(raw, &work).unwrap();
        assert_eq!(
            work.budget().live_bytes(),
            bytes.len() as u64 + span_bytes + 3
        );
        let view = owner.view();
        assert_eq!(view.exact_bytes().as_ptr(), original_pointer);
        assert_eq!(
            view.stanzas.iter().next().unwrap().fields(),
            example_envelope().stanzas[0].fields()
        );
        drop(owner);
        assert_eq!(work.budget().live_bytes(), 3);
        let unwind = std::panic::catch_unwind(|| {
            let mut raw = WorkBytes::zeroed(work.budget(), bytes.len()).unwrap();
            raw.copy_from_slice(&bytes);
            let _owner = WorkEnvelope::decode_with_context(raw, &work).unwrap();
            panic!("synthetic retained-envelope consumer unwind");
        });
        assert!(unwind.is_err());
        assert_eq!(work.budget().live_bytes(), 3);
        drop(retained);
        assert_eq!(work.budget().live_bytes(), 0);
        let foreign = CryptoWorkContext::new(work.policy());
        let mut raw = WorkBytes::zeroed(foreign.budget(), bytes.len()).unwrap();
        raw.copy_from_slice(&bytes);
        assert_eq!(
            WorkEnvelope::decode_with_context(raw, &work)
                .unwrap_err()
                .class(),
            OutcomeClass::PolicyRefused
        );
        assert_eq!(foreign.budget().live_bytes(), 0);
        assert_eq!(work.budget().live_bytes(), 0);
    }

    #[test]
    fn managed_stanza_table_admission_precedes_later_invalid_stanza() {
        let mut sequence = 1_u64.to_be_bytes().to_vec();
        sequence.extend_from_slice(&1_u64.to_be_bytes());
        sequence.push(0); // Count and extent are admissible; this record is malformed.
        let mut record = RecordBuilder::new(RECORD_CRYPTO_ENVELOPE);
        record
            .u16(1, 1)
            .unwrap()
            .u16(2, 1)
            .unwrap()
            .bytes(3, &[4; 32])
            .unwrap()
            .bytes(4, &[5; 32])
            .unwrap()
            .u8(5, 2)
            .unwrap()
            .u8(6, 0)
            .unwrap()
            .u8(7, 1)
            .unwrap()
            .bytes(8, &sequence)
            .unwrap()
            .bytes(9, &[6; 32])
            .unwrap();
        let bytes = record.finish().unwrap();
        for extra in [0, std::mem::size_of::<StanzaSpan>()] {
            let work = CryptoWorkContext::new(super::super::CryptoPolicy {
                max_working_memory_bytes: (bytes.len() + extra) as u64,
                ..super::super::CryptoPolicy::default()
            });
            let mut raw = WorkBytes::zeroed(work.budget(), bytes.len()).unwrap();
            raw.copy_from_slice(&bytes);
            let error = WorkEnvelope::decode_with_context(raw, &work).unwrap_err();
            assert_eq!(error.class() == OutcomeClass::PolicyRefused, extra == 0);
            assert_eq!(work.budget().live_bytes(), 0);
        }
    }

    #[test]
    fn stanza_order_uses_exact_byte_tie_break_and_rejects_duplicates() {
        assert_eq!(
            compare_stanza_keys(([1; 32], b"a"), ([1; 32], b"aa")),
            Ordering::Less
        );
        assert_eq!(
            compare_stanza_keys(([1; 32], b"a"), ([1; 32], b"a")),
            Ordering::Equal
        );
        let stanza = example_envelope().stanzas.remove(0).encode().unwrap();
        let mut sequence = 2_u64.to_be_bytes().to_vec();
        for _ in 0..2 {
            sequence.extend_from_slice(&(stanza.len() as u64).to_be_bytes());
            sequence.extend_from_slice(&stanza);
        }
        assert_eq!(
            scan_stanzas(&sequence, None, |_| Ok(()))
                .unwrap_err()
                .code(),
            ReasonCode::CryptoRecipientStanzaInvalid
        );
    }

    fn assert_cursor_oracle(kind: u16, item: &[u8]) {
        let (record, _) = decode_record(item).unwrap();
        let expected = collection_sort_key(kind, &record, item).unwrap();
        let actual: Vec<_> = collection_cursor(kind, &record, item).unwrap().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn fixed_collection_cursor_matches_all_nine_exact_key_grammars() {
        for (kind, record_type) in [
            (1, 3),
            (1, 4),
            (2, 2),
            (3, 11),
            (4, 12),
            (5, 14),
            (5, 16),
            (6, 18),
            (6, 19),
            (7, 26),
            (8, 22),
            (9, 27),
        ] {
            let mut record = RecordBuilder::new(record_type);
            match (kind, record_type) {
                (1, 3) => {
                    let mut components = Vec::new();
                    for bytes in [b"".as_slice(), b"a", b"a\0", b"abcdef"] {
                        let mut component = RecordBuilder::new(7);
                        component.u8(1, 2).unwrap().bytes(2, bytes).unwrap();
                        components.push(component.finish().unwrap());
                    }
                    record.sequence(1, &components).unwrap();
                }
                (2, _) => {
                    record.u64(1, 0x0102030405060708).unwrap();
                }
                (6, 19) => {
                    record
                        .u8(1, 2)
                        .unwrap()
                        .bytes(2, b"prefix")
                        .unwrap()
                        .utf8(3, "a\u{03bb}")
                        .unwrap();
                }
                (7, _) => {
                    record.u8(1, 1).unwrap();
                }
                _ => {
                    record.bytes(1, b"prefix\0").unwrap();
                }
            }
            let item = record.finish().unwrap();
            assert_cursor_oracle(kind, &item);
            let bytes = sequence_container(kind, std::slice::from_ref(&item)).unwrap();
            let view = SequenceContainerRef::decode(&bytes).unwrap();
            assert_eq!(view.kind(), kind);
            assert_eq!(view.len(), 1);
            let exact_item = view.iter().next().unwrap();
            assert_eq!(exact_item, item);
            assert!(exact_item.as_ptr() >= bytes.as_ptr());
            assert!(exact_item.as_ptr() < bytes.as_ptr().wrapping_add(bytes.len()));
            assert_eq!(
                decode_sequence_container(&bytes).unwrap(),
                (kind, vec![item])
            );
        }
    }

    #[test]
    fn collection_view_keeps_framing_semantic_and_count_admission_precedence() {
        let mut item = RecordBuilder::new(27);
        item.bytes(1, b"a").unwrap();
        let item = item.finish().unwrap();
        let duplicate_then_bad_frame = raw_collection(9, &[item.clone(), item.clone(), vec![0]]);
        let error = SequenceContainerRef::decode(&duplicate_then_bad_frame).unwrap_err();
        assert_ne!(
            error.to_string(),
            private_invalid("EBCS items are duplicate or out of order").to_string()
        );
        let mut bad_key = RecordBuilder::new(27);
        bad_key.u8(1, 1).unwrap();
        let bad_key = bad_key.finish().unwrap();
        let duplicate_then_bad_key = raw_collection(9, &[item.clone(), item.clone(), bad_key]);
        assert_eq!(
            SequenceContainerRef::decode(&duplicate_then_bad_key).unwrap_err(),
            private_invalid("EBCS items are duplicate or out of order")
        );
        assert_eq!(
            validate_sequence_semantics(9, &[item.clone(), item]).unwrap_err(),
            private_invalid("EBCS items are duplicate or out of order")
        );
        let refusal = super::super::resource_refused("synthetic admission observer");
        let input = raw_collection(9, &[vec![0]]);
        let result =
            SequenceContainerRef::decode_with_admission(&input, |admission| match admission {
                CollectionAdmission::Count(1) => Err(refusal.clone()),
                _ => panic!("item reached before rejected count admission"),
            });
        assert_eq!(result.unwrap_err(), refusal);
        let mut admitted = 0;
        let error = SequenceContainerRef::decode_with_admission(&input, |admission| {
            if let CollectionAdmission::Item(_) = admission {
                admitted += 1;
            }
            Ok(())
        })
        .unwrap_err();
        assert_eq!(
            admitted, 1,
            "compatibility copy is admitted before generic item decode"
        );
        assert_eq!(error, decode_record(&[0]).unwrap_err());
    }

    #[test]
    fn object_hash_streams_the_exact_existing_t1_bytes() {
        use sha2::{Digest as _, Sha256};
        for length in [0, 1, 17, 64, 1024, 65_536] {
            let bytes: Vec<u8> = (0..length).map(|i| (i % 251) as u8).collect();
            let expected: [u8; 32] =
                Sha256::digest(t1("entrybound/encrypted-object/v1", &[&bytes]).unwrap()).into();
            assert_eq!(encrypted_object_id(&bytes).unwrap(), expected);
        }
        for label in ["", "\u{03bb}"] {
            assert_eq!(
                hash_t1_single(label, &[]).unwrap_err(),
                t1(label, &[&[]]).unwrap_err()
            );
        }
    }

    #[test]
    fn private_fragment_borrows_its_exact_record_payload() {
        let bytes = encode_private_fragment(&[3; 32], 100, 1, 3, 12, &[5, 6, 7]).unwrap();
        let fragment = decode_private_fragment_ref(&bytes).unwrap();
        let (ordinary, consumed) = crate::canonical::decode_record(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        let payload = ordinary.field(6).unwrap().as_bytes().unwrap();
        assert_eq!(fragment.bytes, payload);
        assert!(std::ptr::eq(fragment.bytes.as_ptr(), payload.as_ptr()));
        assert_eq!(
            (
                fragment.object_id,
                fragment.total_len,
                fragment.index,
                fragment.count,
                fragment.offset
            ),
            ([3; 32], 100, 1, 3, 12)
        );
        let mut tail = bytes;
        tail.push(0);
        assert_eq!(
            decode_private_fragment_ref(&tail).unwrap_err().code(),
            ReasonCode::CryptoPrivateObjectInvalid
        );
        let mut zero_count = RecordBuilder::new(RECORD_PRIVATE_FRAGMENT);
        zero_count
            .bytes(1, &[3; 32])
            .unwrap()
            .u64(2, 100)
            .unwrap()
            .u32(3, 0)
            .unwrap()
            .u32(4, 0)
            .unwrap()
            .u64(5, 0)
            .unwrap()
            .bytes(6, &[5])
            .unwrap();
        assert_eq!(
            decode_private_fragment_ref(&zero_count.finish().unwrap())
                .unwrap_err()
                .code(),
            ReasonCode::CryptoPrivateObjectInvalid
        );
    }

    #[test]
    fn t1_and_sequence_are_closed_and_canonical() {
        let transcript = t1("x", &[b"a", b""]).unwrap();
        assert_eq!(
            decode_t1(&transcript, "x", 2).unwrap(),
            vec![b"a".as_slice(), b"".as_slice()]
        );
        let empty = sequence_container(COLLECTION_MANIFEST, &[]).unwrap();
        assert_eq!(hex(&empty), "4542435300010001000000000000000000000000");
        assert_eq!(
            decode_sequence_container(&empty).unwrap(),
            (COLLECTION_MANIFEST, vec![])
        );
    }

    #[test]
    fn counted_collections_refuse_impossible_extent_before_reserving() {
        let mut transcript = 1_u16.to_be_bytes().to_vec();
        transcript.push(b'x');
        transcript.extend_from_slice(&u16::MAX.to_be_bytes());
        assert_eq!(
            decode_t1(&transcript, "x", usize::from(u16::MAX))
                .unwrap_err()
                .code(),
            ReasonCode::CryptoPrivateObjectInvalid
        );

        let stanzas = 1_024_u64.to_be_bytes().to_vec();
        assert_eq!(
            decode_stanza_sequence(&stanzas).unwrap_err().code(),
            ReasonCode::CryptoRecipientStanzaInvalid
        );
        let mut one_invalid_stanza = 1_u64.to_be_bytes().to_vec();
        one_invalid_stanza.extend_from_slice(&1_u64.to_be_bytes());
        one_invalid_stanza.push(0);
        assert!(decode_stanza_sequence(&one_invalid_stanza).is_err());

        let mut sequence = b"EBCS".to_vec();
        sequence.extend_from_slice(&1_u16.to_be_bytes());
        sequence.extend_from_slice(&COLLECTION_MANIFEST.to_be_bytes());
        sequence.extend_from_slice(&0_u32.to_be_bytes());
        sequence.extend_from_slice(&1_000_000_u64.to_be_bytes());
        assert_eq!(sequence.len(), 20);
        assert_eq!(
            decode_sequence_container(&sequence).unwrap_err().code(),
            ReasonCode::CryptoPrivateObjectInvalid
        );
    }

    #[test]
    fn envelope_caller_limits_apply_to_public_stanza_framing() {
        let stanza = RecipientStanza {
            stanza_type: 2,
            protection_class: 2,
            stanza_id: [1; 16],
            recipient_hint: [0; 16],
            method_parameters: vec![0; PASSWORD_METHOD_LEN],
            encapsulation: Vec::new(),
            wrap_nonce: [2; 12],
            wrapped_afk: [3; 48],
        };
        let encoded_stanza_len = stanza.encode().unwrap().len() as u64;
        let envelope = CryptoEnvelope {
            archive_id: [4; 32],
            commitment: [5; 32],
            protection_policy: 2,
            padding_mode: 0,
            boundary_mode: 1,
            stanzas: vec![stanza],
            envelope_mac: [6; 32],
        };
        let encoded = envelope.encode().unwrap();
        assert_eq!(CryptoEnvelope::decode(&encoded).unwrap(), envelope);
        assert_eq!(
            CryptoEnvelope::decode_with_policy(&encoded, super::super::CryptoPolicy::default())
                .unwrap(),
            envelope
        );

        for limited in [
            super::super::CryptoPolicy {
                max_stanzas: 0,
                ..super::super::CryptoPolicy::default()
            },
            super::super::CryptoPolicy {
                max_stanza_bytes: encoded_stanza_len - 1,
                ..super::super::CryptoPolicy::default()
            },
        ] {
            let error = CryptoEnvelope::decode_with_policy(&encoded, limited).unwrap_err();
            assert_eq!(error.class(), OutcomeClass::PolicyRefused);
            assert_eq!(error.code(), ReasonCode::CryptoResourcePolicyRefused);
        }
    }

    #[test]
    fn all_nine_collection_kinds_have_explicit_dispatch() {
        let cases = [
            (COLLECTION_MANIFEST, 3),
            (COLLECTION_TRANSFORM_PLANS, 2),
            (COLLECTION_DICTIONARIES, 11),
            (COLLECTION_CHUNK_GROUPS, 12),
            (COLLECTION_RECONSTRUCTION_DATA, 14),
            (COLLECTION_RECONSTRUCTION_REGIONS, 18),
            (COLLECTION_SIGNATURES, 26),
            (COLLECTION_RECIPIENT_DIRECTORY, 22),
            (COLLECTION_INDEX, 27),
        ];
        for (collection, record_type) in cases {
            let mut record = RecordBuilder::new(record_type);
            match collection {
                COLLECTION_MANIFEST => {
                    record.sequence(1, &[]).unwrap();
                }
                COLLECTION_TRANSFORM_PLANS => {
                    record.u64(1, 1).unwrap();
                }
                COLLECTION_SIGNATURES => {
                    record.u8(1, 1).unwrap();
                }
                COLLECTION_RECIPIENT_DIRECTORY => {
                    record.bytes(1, &[1; 16]).unwrap();
                }
                _ => {
                    record.bytes(1, &[1; 32]).unwrap();
                }
            }
            let item = record.finish().unwrap();
            let sequence = sequence_container(collection, std::slice::from_ref(&item)).unwrap();
            let object = private_object(PRIVATE_OBJECT_SEQUENCE, &sequence).unwrap();
            let (kind, payload) = decode_private_object(&object).unwrap();
            assert_eq!(kind, PRIVATE_OBJECT_SEQUENCE);
            assert_eq!(
                decode_sequence_container(payload).unwrap(),
                (collection, vec![item])
            );
        }
    }

    #[test]
    fn sequence_rejects_duplicate_or_reordered_items_and_truncation() {
        let mut first = RecordBuilder::new(RECORD_RECIPIENT_DIRECTORY);
        first.bytes(1, &[1; 16]).unwrap();
        let first = first.finish().unwrap();
        assert!(
            sequence_container(
                COLLECTION_RECIPIENT_DIRECTORY,
                &[first.clone(), first.clone()]
            )
            .is_err()
        );

        let mut second = RecordBuilder::new(RECORD_RECIPIENT_DIRECTORY);
        second.bytes(1, &[0; 16]).unwrap();
        let second = second.finish().unwrap();
        assert!(
            sequence_container(COLLECTION_RECIPIENT_DIRECTORY, &[first.clone(), second]).is_err()
        );

        let mut sequence =
            sequence_container(COLLECTION_RECIPIENT_DIRECTORY, std::slice::from_ref(&first))
                .unwrap();
        sequence.pop();
        assert!(decode_sequence_container(&sequence).is_err());
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
