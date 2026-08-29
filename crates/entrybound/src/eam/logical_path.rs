use std::cmp::Ordering;
use std::fmt;

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};

/// The declared encoding of a native path component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathEncoding {
    /// UTF-8 without normalization.
    Utf8,
}

/// One validated component of a structural logical path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathComponent {
    bytes: Box<[u8]>,
    encoding: PathEncoding,
}

impl PathComponent {
    /// Validates and constructs a path component.
    pub fn new(bytes: impl Into<Box<[u8]>>, encoding: PathEncoding) -> Result<Self> {
        let bytes = bytes.into();
        if bytes.is_empty() || bytes.contains(&0) || bytes.contains(&b'/') {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::InvalidPathComponent,
                "components must be non-empty and contain neither NUL nor slash",
            ));
        }
        if encoding != PathEncoding::Utf8 || std::str::from_utf8(&bytes).is_err() {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::InvalidPathComponent,
                "the bootstrap subset requires valid UTF-8 components",
            ));
        }
        match bytes.as_ref() {
            b"." => {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::DotComponent,
                    "'.' cannot be represented as a LogicalPath component",
                ));
            }
            b".." => {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::DotDotComponent,
                    "'..' cannot be represented as a LogicalPath component",
                ));
            }
            _ => {}
        }
        Ok(Self { bytes, encoding })
    }

    /// Returns the exact component bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the component's declared encoding.
    #[must_use]
    pub const fn encoding(&self) -> PathEncoding {
        self.encoding
    }
}

/// A non-empty, relative sequence of validated path components.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalPath {
    components: Box<[PathComponent]>,
}

impl LogicalPath {
    /// Constructs a logical path from already separated UTF-8 components.
    pub fn from_utf8<I, S>(components: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let components = components
            .into_iter()
            .map(|value| PathComponent::new(value.as_ref().as_bytes(), PathEncoding::Utf8))
            .collect::<Result<Vec<_>>>()?;
        Self::new(components)
    }

    /// Constructs a logical path from validated component values.
    pub fn new(components: Vec<PathComponent>) -> Result<Self> {
        if components.is_empty() {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::InvalidPathComponent,
                "a LogicalPath must contain at least one component",
            ));
        }
        Ok(Self {
            components: components.into_boxed_slice(),
        })
    }

    /// Returns the ordered path components.
    #[must_use]
    pub fn components(&self) -> &[PathComponent] {
        &self.components
    }

    /// Returns the number of structural components.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.components.len()
    }

    pub(crate) fn prefix(&self, depth: usize) -> Self {
        Self {
            components: self.components[..depth].to_vec().into_boxed_slice(),
        }
    }
}

impl Ord for LogicalPath {
    fn cmp(&self, other: &Self) -> Ordering {
        for (left, right) in self.components.iter().zip(other.components.iter()) {
            let ordering = left
                .bytes
                .len()
                .cmp(&right.bytes.len())
                .then_with(|| left.bytes.cmp(&right.bytes));
            if ordering != Ordering::Equal {
                return ordering;
            }
        }
        self.components.len().cmp(&other.components.len())
    }
}

impl PartialOrd for LogicalPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for LogicalPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, component) in self.components.iter().enumerate() {
            if index != 0 {
                formatter.write_str("/")?;
            }
            formatter.write_str(std::str::from_utf8(&component.bytes).map_err(|_| fmt::Error)?)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::LogicalPath;
    use crate::diagnostics::ReasonCode;

    #[test]
    fn dot_and_dot_dot_are_unrepresentable() {
        let dot = LogicalPath::from_utf8(["."]).unwrap_err();
        let dot_dot = LogicalPath::from_utf8([".."]).unwrap_err();
        assert_eq!(dot.code(), ReasonCode::DotComponent);
        assert_eq!(dot_dot.code(), ReasonCode::DotDotComponent);
    }

    #[test]
    fn canonical_order_uses_length_prefixed_component_bytes() {
        let mut paths = [
            LogicalPath::from_utf8(["aa"]).unwrap(),
            LogicalPath::from_utf8(["b"]).unwrap(),
            LogicalPath::from_utf8(["a", "z"]).unwrap(),
            LogicalPath::from_utf8(["a"]).unwrap(),
        ];
        paths.sort();
        let rendered = paths.map(|path| path.to_string());
        assert_eq!(rendered, ["a", "a/z", "b", "aa"]);
    }
}
