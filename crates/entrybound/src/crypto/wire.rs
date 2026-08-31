//! Canonical crypto-v1 transcripts and private-object grammars.

use std::cmp::Ordering;

use crate::canonical::{RecordBuilder, decode_record};
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
    let mut fields = Vec::with_capacity(count);
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

pub(crate) fn decode_stanza_sequence(input: &[u8]) -> Result<Vec<RecipientStanza>> {
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
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let len = to_usize(take_u64(input, &mut cursor)?)?;
        if len == 0 || len > 65_536 {
            return Err(stanza_invalid(
                "recipient stanza length is outside v1 bounds",
            ));
        }
        values.push(RecipientStanza::decode(take(input, &mut cursor, len)?)?);
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

    pub(crate) fn decode(input: &[u8]) -> Result<Self> {
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
            stanzas: decode_stanza_sequence(record.field(8)?.as_bytes()?)?,
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
    validate_sequence_semantics(kind, items)?;
    Ok(out)
}

pub(crate) fn decode_sequence_container(input: &[u8]) -> Result<(u16, Vec<Vec<u8>>)> {
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
    let mut cursor = 20usize;
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        let len = to_usize(take_u64(input, &mut cursor)?)?;
        if len == 0 || len > MAX_SEQUENCE_ITEM_BYTES {
            return Err(private_invalid("EBCS item length is outside v1 bounds"));
        }
        let item = take(input, &mut cursor, len)?.to_vec();
        let (_, consumed) = decode_record(&item)?;
        if consumed != item.len() {
            return Err(private_invalid("EBCS item has trailing bytes"));
        }
        items.push(item);
    }
    if cursor != input.len() {
        return Err(private_invalid("EBCS has trailing or truncated bytes"));
    }
    if sequence_container(kind, &items)? != input {
        return Err(private_invalid("EBCS is not canonical"));
    }
    Ok((kind, items))
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

fn collection_sort_key(
    kind: u16,
    record: &crate::canonical::Record<'_>,
    exact: &[u8],
) -> Result<Vec<u8>> {
    let mut key = record.kind.to_be_bytes().to_vec();
    match kind {
        COLLECTION_MANIFEST if record.kind == 3 => {
            for component_bytes in record.field(1)?.as_sequence()? {
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
pub(crate) struct PrivateFragment {
    pub object_id: [u8; 32],
    pub total_len: u64,
    pub index: u32,
    pub count: u32,
    pub offset: u64,
    pub bytes: Vec<u8>,
}

pub(crate) fn decode_private_fragment(input: &[u8]) -> Result<PrivateFragment> {
    let (record, consumed) = decode_record(input)?;
    if consumed != input.len() || record.kind != RECORD_PRIVATE_FRAGMENT {
        return Err(private_invalid(
            "DATA plaintext is not one PrivateFragment record",
        ));
    }
    record.expect_tags(&[1, 2, 3, 4, 5, 6], &[])?;
    let value = PrivateFragment {
        object_id: exact(record.field(1)?.as_bytes()?)?,
        total_len: record.field(2)?.as_u64()?,
        index: record.field(3)?.as_u32()?,
        count: record.field(4)?.as_u32()?,
        offset: record.field(5)?.as_u64()?,
        bytes: record.field(6)?.as_bytes()?.to_vec(),
    };
    if value.count == 0 || value.index >= value.count || value.bytes.is_empty() {
        return Err(private_invalid("PrivateFragment cardinality is invalid"));
    }
    Ok(value)
}

pub(crate) fn encrypted_object_id(bytes: &[u8]) -> Result<[u8; 32]> {
    use sha2::{Digest as _, Sha256};
    Ok(Sha256::digest(t1("entrybound/encrypted-object/v1", &[bytes])?).into())
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
