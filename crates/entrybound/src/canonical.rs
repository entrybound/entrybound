//! Central canonical record and TLV encoding for `ecf/bootstrap-v1`.

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};

pub(crate) const RECORD_HEADER_LEN: usize = 16;
const FIELD_HEADER_LEN: usize = 12;
const RECORD_VERSION_V1: u16 = 1;
const RECORD_VERSION_V2: u16 = 2;
const MAX_SEQUENCE_ITEMS: u64 = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum FieldType {
    U8 = 1,
    U16 = 2,
    U32 = 3,
    U64 = 4,
    I64 = 5,
    Bool = 6,
    Bytes = 7,
    Utf8 = 8,
    Sequence = 9,
}

impl FieldType {
    fn from_byte(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::U8),
            2 => Ok(Self::U16),
            3 => Ok(Self::U32),
            4 => Ok(Self::U64),
            5 => Ok(Self::I64),
            6 => Ok(Self::Bool),
            7 => Ok(Self::Bytes),
            8 => Ok(Self::Utf8),
            9 => Ok(Self::Sequence),
            _ => Err(noncanonical("unknown bootstrap field type")),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RecordBuilder {
    kind: u16,
    version: u16,
    payload: Vec<u8>,
    last_tag: Option<u16>,
}

impl RecordBuilder {
    pub(crate) const fn new(kind: u16) -> Self {
        Self {
            kind,
            version: RECORD_VERSION_V1,
            payload: Vec::new(),
            last_tag: None,
        }
    }

    /// Constructs one explicitly versioned canonical record.
    ///
    /// Version 2 is currently assigned only to Descriptor record kind 1. The
    /// ordinary constructor remains frozen to version 1 for every historical
    /// writer.
    pub(crate) const fn new_version(kind: u16, version: u16) -> Self {
        Self {
            kind,
            version,
            payload: Vec::new(),
            last_tag: None,
        }
    }

    fn field(&mut self, tag: u16, field_type: FieldType, value: &[u8]) -> Result<&mut Self> {
        if let Some(previous) = self.last_tag {
            if tag == previous {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::DuplicateSemanticDeclaration,
                    format!("duplicate canonical field tag {tag}"),
                ));
            }
            if tag < previous {
                return Err(noncanonical(
                    "canonical field tags must be strictly increasing",
                ));
            }
        }
        self.last_tag = Some(tag);
        self.payload.extend_from_slice(&tag.to_be_bytes());
        self.payload.push(field_type as u8);
        self.payload.push(0);
        self.payload.extend_from_slice(
            &u64::try_from(value.len())
                .map_err(|_| resource_limit("field exceeds u64"))?
                .to_be_bytes(),
        );
        self.payload.extend_from_slice(value);
        Ok(self)
    }

    pub(crate) fn u8(&mut self, tag: u16, value: u8) -> Result<&mut Self> {
        self.field(tag, FieldType::U8, &[value])
    }

    pub(crate) fn u16(&mut self, tag: u16, value: u16) -> Result<&mut Self> {
        self.field(tag, FieldType::U16, &value.to_be_bytes())
    }

    pub(crate) fn u32(&mut self, tag: u16, value: u32) -> Result<&mut Self> {
        self.field(tag, FieldType::U32, &value.to_be_bytes())
    }

    pub(crate) fn u64(&mut self, tag: u16, value: u64) -> Result<&mut Self> {
        self.field(tag, FieldType::U64, &value.to_be_bytes())
    }

    pub(crate) fn i64(&mut self, tag: u16, value: i64) -> Result<&mut Self> {
        self.field(tag, FieldType::I64, &value.to_be_bytes())
    }

    pub(crate) fn bool(&mut self, tag: u16, value: bool) -> Result<&mut Self> {
        self.field(tag, FieldType::Bool, &[u8::from(value)])
    }

    pub(crate) fn bytes(&mut self, tag: u16, value: &[u8]) -> Result<&mut Self> {
        self.field(tag, FieldType::Bytes, value)
    }

    pub(crate) fn utf8(&mut self, tag: u16, value: &str) -> Result<&mut Self> {
        self.field(tag, FieldType::Utf8, value.as_bytes())
    }

    pub(crate) fn sequence(&mut self, tag: u16, items: &[Vec<u8>]) -> Result<&mut Self> {
        let value = encode_sequence(items)?;
        self.field(tag, FieldType::Sequence, &value)
    }

    pub(crate) fn finish(self) -> Result<Vec<u8>> {
        if self.version == 0
            || self.version > RECORD_VERSION_V2
            || (self.version == RECORD_VERSION_V2 && self.kind != 1)
        {
            return Err(noncanonical("unsupported canonical record version"));
        }
        let payload_len = u64::try_from(self.payload.len())
            .map_err(|_| resource_limit("record payload exceeds u64"))?;
        let mut record = Vec::with_capacity(RECORD_HEADER_LEN + self.payload.len());
        record.extend_from_slice(&self.kind.to_be_bytes());
        record.extend_from_slice(&self.version.to_be_bytes());
        record.extend_from_slice(&0_u32.to_be_bytes());
        record.extend_from_slice(&payload_len.to_be_bytes());
        record.extend_from_slice(&self.payload);
        Ok(record)
    }
}

pub(crate) fn encode_sequence(items: &[Vec<u8>]) -> Result<Vec<u8>> {
    let count = u64::try_from(items.len()).map_err(|_| resource_limit("sequence exceeds u64"))?;
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&count.to_be_bytes());
    for item in items {
        encoded.extend_from_slice(
            &u64::try_from(item.len())
                .map_err(|_| resource_limit("sequence item exceeds u64"))?
                .to_be_bytes(),
        );
        encoded.extend_from_slice(item);
    }
    Ok(encoded)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Field<'a> {
    pub(crate) tag: u16,
    field_type: FieldType,
    value: &'a [u8],
}

impl<'a> Field<'a> {
    fn require(self, expected: FieldType) -> Result<&'a [u8]> {
        if self.field_type != expected {
            return Err(noncanonical("canonical field has the wrong type"));
        }
        Ok(self.value)
    }

    pub(crate) fn as_u8(self) -> Result<u8> {
        let value = self.require(FieldType::U8)?;
        if value.len() != 1 {
            return Err(noncanonical("u8 field must contain exactly one byte"));
        }
        Ok(value[0])
    }

    pub(crate) fn as_u16(self) -> Result<u16> {
        Ok(u16::from_be_bytes(exact(self.require(FieldType::U16)?)?))
    }

    pub(crate) fn as_u32(self) -> Result<u32> {
        Ok(u32::from_be_bytes(exact(self.require(FieldType::U32)?)?))
    }

    pub(crate) fn as_u64(self) -> Result<u64> {
        Ok(u64::from_be_bytes(exact(self.require(FieldType::U64)?)?))
    }

    pub(crate) fn as_i64(self) -> Result<i64> {
        Ok(i64::from_be_bytes(exact(self.require(FieldType::I64)?)?))
    }

    pub(crate) fn as_bool(self) -> Result<bool> {
        match self.require(FieldType::Bool)? {
            [0] => Ok(false),
            [1] => Ok(true),
            _ => Err(noncanonical("boolean field must be exactly 00 or 01")),
        }
    }

    pub(crate) fn as_bytes(self) -> Result<&'a [u8]> {
        self.require(FieldType::Bytes)
    }

    pub(crate) fn as_utf8(self) -> Result<&'a str> {
        std::str::from_utf8(self.require(FieldType::Utf8)?)
            .map_err(|_| noncanonical("UTF-8 field contains invalid bytes"))
    }

    pub(crate) fn as_sequence(self) -> Result<Vec<&'a [u8]>> {
        decode_sequence(self.require(FieldType::Sequence)?)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Record<'a> {
    pub(crate) kind: u16,
    pub(crate) version: u16,
    fields: Vec<Field<'a>>,
}

impl<'a> Record<'a> {
    pub(crate) fn expect_tags(&self, required: &[u16], optional: &[u16]) -> Result<()> {
        self.expect_versioned_tags(RECORD_VERSION_V1, required, optional)
    }

    pub(crate) fn expect_versioned_tags(
        &self,
        version: u16,
        required: &[u16],
        optional: &[u16],
    ) -> Result<()> {
        if self.version != version {
            return Err(noncanonical("unsupported canonical record version"));
        }
        for tag in required {
            if !self.fields.iter().any(|field| field.tag == *tag) {
                return Err(noncanonical("canonical record is missing a required field"));
            }
        }
        if let Some(field) = self
            .fields
            .iter()
            .find(|field| !required.contains(&field.tag) && !optional.contains(&field.tag))
        {
            return Err(noncanonical(format!(
                "unknown bootstrap field tag {}",
                field.tag
            )));
        }
        Ok(())
    }

    pub(crate) fn field(&self, tag: u16) -> Result<Field<'a>> {
        self.optional_field(tag)
            .ok_or_else(|| noncanonical(format!("canonical record is missing field tag {tag}")))
    }

    pub(crate) fn optional_field(&self, tag: u16) -> Option<Field<'a>> {
        self.fields.iter().copied().find(|field| field.tag == tag)
    }
}

pub(crate) fn decode_record(input: &[u8]) -> Result<(Record<'_>, usize)> {
    if input.len() < RECORD_HEADER_LEN {
        return Err(structure("record header is truncated"));
    }
    let kind = u16::from_be_bytes(exact(&input[0..2])?);
    let version = u16::from_be_bytes(exact(&input[2..4])?);
    if version == 0 || version > RECORD_VERSION_V2 || (version == RECORD_VERSION_V2 && kind != 1) {
        return Err(noncanonical("unsupported canonical record version"));
    }
    if input[4..8] != [0; 4] {
        return Err(noncanonical("record flags must be zero"));
    }
    let payload_len = to_usize(u64::from_be_bytes(exact(&input[8..16])?))?;
    let total_len = RECORD_HEADER_LEN
        .checked_add(payload_len)
        .ok_or_else(|| structure("record length overflow"))?;
    if total_len > input.len() {
        return Err(structure("record payload length exceeds enclosing bytes"));
    }

    let mut fields = Vec::new();
    let mut cursor = RECORD_HEADER_LEN;
    let mut previous = None;
    while cursor < total_len {
        if total_len - cursor < FIELD_HEADER_LEN {
            return Err(structure("canonical field header is truncated"));
        }
        let tag = u16::from_be_bytes(exact(&input[cursor..cursor + 2])?);
        let field_type = FieldType::from_byte(input[cursor + 2])?;
        if input[cursor + 3] != 0 {
            return Err(noncanonical("field flags must be zero"));
        }
        if let Some(last) = previous {
            if tag == last {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::DuplicateSemanticDeclaration,
                    format!("duplicate canonical field tag {tag}"),
                ));
            }
            if tag < last {
                return Err(noncanonical("canonical field tags are out of order"));
            }
        }
        previous = Some(tag);
        let value_len = to_usize(u64::from_be_bytes(exact(
            &input[cursor + 4..cursor + FIELD_HEADER_LEN],
        )?))?;
        let value_start = cursor + FIELD_HEADER_LEN;
        let value_end = value_start
            .checked_add(value_len)
            .ok_or_else(|| structure("field length overflow"))?;
        if value_end > total_len {
            return Err(structure("field length exceeds its record payload"));
        }
        let value = &input[value_start..value_end];
        validate_minimal_value(field_type, value)?;
        fields.push(Field {
            tag,
            field_type,
            value,
        });
        cursor = value_end;
    }
    Ok((
        Record {
            kind,
            version,
            fields,
        },
        total_len,
    ))
}

pub(crate) fn decode_record_stream(input: &[u8]) -> Result<Vec<Record<'_>>> {
    let mut records = Vec::new();
    let mut cursor = 0;
    while cursor < input.len() {
        let (record, consumed) = decode_record(&input[cursor..])?;
        records.push(record);
        cursor = cursor
            .checked_add(consumed)
            .ok_or_else(|| resource_limit("record stream offset overflow"))?;
    }
    Ok(records)
}

fn decode_sequence(value: &[u8]) -> Result<Vec<&[u8]>> {
    if value.len() < 8 {
        return Err(structure("sequence count is truncated"));
    }
    let count = u64::from_be_bytes(exact(&value[..8])?);
    if count > MAX_SEQUENCE_ITEMS {
        return Err(resource_limit(
            "sequence item count exceeds bootstrap policy",
        ));
    }
    let capacity = to_usize(count)?;
    let mut items = Vec::with_capacity(capacity);
    let mut cursor = 8;
    for _ in 0..count {
        if value.len() - cursor < 8 {
            return Err(structure("sequence item length is truncated"));
        }
        let item_len = to_usize(u64::from_be_bytes(exact(&value[cursor..cursor + 8])?))?;
        cursor += 8;
        let end = cursor
            .checked_add(item_len)
            .ok_or_else(|| structure("sequence item length overflow"))?;
        if end > value.len() {
            return Err(structure("sequence item exceeds its enclosing field"));
        }
        items.push(&value[cursor..end]);
        cursor = end;
    }
    if cursor != value.len() {
        return Err(noncanonical("sequence contains trailing bytes"));
    }
    Ok(items)
}

fn validate_minimal_value(field_type: FieldType, value: &[u8]) -> Result<()> {
    let expected = match field_type {
        FieldType::U8 | FieldType::Bool => Some(1),
        FieldType::U16 => Some(2),
        FieldType::U32 => Some(4),
        FieldType::U64 | FieldType::I64 => Some(8),
        FieldType::Bytes | FieldType::Utf8 | FieldType::Sequence => None,
    };
    if expected.is_some_and(|size| value.len() != size) {
        return Err(noncanonical("fixed-width field has a noncanonical length"));
    }
    if field_type == FieldType::Bool && !matches!(value, [0] | [1]) {
        return Err(noncanonical("boolean field must be exactly 00 or 01"));
    }
    if field_type == FieldType::Utf8 && std::str::from_utf8(value).is_err() {
        return Err(noncanonical("UTF-8 field contains invalid bytes"));
    }
    Ok(())
}

fn exact<const N: usize>(value: &[u8]) -> Result<[u8; N]> {
    value
        .try_into()
        .map_err(|_| noncanonical(format!("expected exactly {N} bytes")))
}

fn to_usize(value: u64) -> Result<usize> {
    usize::try_from(value).map_err(|_| resource_limit("length does not fit this platform"))
}

fn noncanonical(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::NoncanonicalEncoding,
        detail,
    )
}

fn structure(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Corrupt, ReasonCode::SectionStructure, detail)
}

fn resource_limit(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ResourceLimit,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use super::{FieldType, RECORD_HEADER_LEN, RecordBuilder, decode_record};
    use crate::diagnostics::ReasonCode;

    #[test]
    fn record_round_trip_is_deterministic() {
        let mut builder = RecordBuilder::new(7);
        builder.u8(1, 4).unwrap().utf8(2, "entry").unwrap();
        let bytes = builder.finish().unwrap();
        assert_eq!(&bytes[2..4], &1_u16.to_be_bytes());
        let (record, consumed) = decode_record(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(record.kind, 7);
        assert_eq!(record.version, 1);
        assert_eq!(record.field(1).unwrap().as_u8().unwrap(), 4);
        assert_eq!(record.field(2).unwrap().as_utf8().unwrap(), "entry");
    }

    #[test]
    fn only_descriptor_may_use_record_version_two() {
        let mut descriptor = RecordBuilder::new_version(1, 2);
        descriptor.u8(1, 7).unwrap();
        let bytes = descriptor.finish().unwrap();
        let (record, _) = decode_record(&bytes).unwrap();
        assert_eq!(record.kind, 1);
        assert_eq!(record.version, 2);

        let mut unrelated = RecordBuilder::new_version(2, 2);
        unrelated.u8(1, 7).unwrap();
        assert_eq!(
            unrelated.finish().unwrap_err().code(),
            ReasonCode::NoncanonicalEncoding
        );

        let mut version_three = bytes;
        version_three[2..4].copy_from_slice(&3_u16.to_be_bytes());
        assert_eq!(
            decode_record(&version_three).unwrap_err().code(),
            ReasonCode::NoncanonicalEncoding
        );
    }

    #[test]
    fn out_of_order_tags_are_rejected() {
        let mut builder = RecordBuilder::new(1);
        builder.u8(1, 1).unwrap().u8(2, 2).unwrap();
        let mut bytes = builder.finish().unwrap();
        bytes[RECORD_HEADER_LEN] = 0;
        bytes[RECORD_HEADER_LEN + 1] = 2;
        bytes[RECORD_HEADER_LEN + 13] = 0;
        bytes[RECORD_HEADER_LEN + 14] = 1;
        let error = decode_record(&bytes).unwrap_err();
        assert_eq!(error.code(), ReasonCode::NoncanonicalEncoding);
    }

    #[test]
    fn duplicate_tags_are_distinguished() {
        let mut builder = RecordBuilder::new(1);
        builder.u8(1, 1).unwrap().u8(2, 2).unwrap();
        let mut bytes = builder.finish().unwrap();
        bytes[RECORD_HEADER_LEN + 13] = 0;
        bytes[RECORD_HEADER_LEN + 14] = 1;
        let error = decode_record(&bytes).unwrap_err();
        assert_eq!(error.code(), ReasonCode::DuplicateSemanticDeclaration);
    }

    #[test]
    fn malformed_record_length_is_corrupt() {
        let mut builder = RecordBuilder::new(1);
        builder.u8(1, 1).unwrap();
        let mut bytes = builder.finish().unwrap();
        bytes[8..16].copy_from_slice(&u64::MAX.to_be_bytes());
        let error = decode_record(&bytes).unwrap_err();
        assert_eq!(error.code(), ReasonCode::SectionStructure);
    }

    #[test]
    fn non_minimal_integer_is_rejected() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(&14_u64.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.push(FieldType::U8 as u8);
        bytes.push(0);
        bytes.extend_from_slice(&2_u64.to_be_bytes());
        bytes.extend_from_slice(&[0, 1]);
        let error = decode_record(&bytes).unwrap_err();
        assert_eq!(error.code(), ReasonCode::NoncanonicalEncoding);
    }
}
