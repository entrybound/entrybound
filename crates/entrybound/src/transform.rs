//! Governed reversible structural transforms used by recorded TransformSteps.
//!
//! Archive strings are matched only against this closed registry. Every
//! registered transform is length-preserving and bijective for all byte input.

use std::collections::BTreeSet;

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::TransformStep;
use crate::ecf::FEATURE_CODEC_TRANSFORM_V1;

pub(crate) const DELTA8_ID: &str = "delta8/v1";
pub(crate) const BYTE_SHUFFLE_ID: &str = "byte-shuffle/v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReversibilityClass {
    BijectiveAllByteStrings,
}

pub(crate) struct TransformRegistration {
    pub identifier: &'static str,
    pub format_version: u16,
    pub required_feature: u64,
    pub reversibility: ReversibilityClass,
    validate: fn(&[u8]) -> Result<()>,
    forward: fn(&[u8], &[u8]) -> Result<Vec<u8>>,
    inverse: fn(&[u8], &[u8]) -> Result<Vec<u8>>,
}

static TRANSFORMS: [TransformRegistration; 2] = [
    TransformRegistration {
        identifier: DELTA8_ID,
        format_version: 1,
        required_feature: FEATURE_CODEC_TRANSFORM_V1,
        reversibility: ReversibilityClass::BijectiveAllByteStrings,
        validate: validate_delta8,
        forward: delta8_forward,
        inverse: delta8_inverse,
    },
    TransformRegistration {
        identifier: BYTE_SHUFFLE_ID,
        format_version: 1,
        required_feature: FEATURE_CODEC_TRANSFORM_V1,
        reversibility: ReversibilityClass::BijectiveAllByteStrings,
        validate: validate_shuffle,
        forward: shuffle_forward,
        inverse: shuffle_inverse,
    },
];

pub(crate) fn delta8_step() -> TransformStep {
    TransformStep {
        transform_id: DELTA8_ID.to_owned(),
        parameters: Box::default(),
    }
}

pub(crate) fn byte_shuffle_step(width: u8) -> Result<TransformStep> {
    let step = TransformStep {
        transform_id: BYTE_SHUFFLE_ID.to_owned(),
        parameters: Box::from([width]),
    };
    validate_step(&step)?;
    Ok(step)
}

pub(crate) fn validate_pipeline(steps: &[TransformStep]) -> Result<()> {
    let mut identifiers = BTreeSet::new();
    for step in steps {
        let registration = registration(&step.transform_id)?;
        (registration.validate)(&step.parameters)?;
        if !identifiers.insert(step.transform_id.as_str()) {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                format!("duplicate TransformStep '{}'", step.transform_id),
            ));
        }
    }
    Ok(())
}

pub(crate) fn required_features(steps: &[TransformStep]) -> Result<u64> {
    steps.iter().try_fold(0_u64, |features, step| {
        let registration = registration(&step.transform_id)?;
        Ok(features | registration.required_feature)
    })
}

pub(crate) fn forward_pipeline(steps: &[TransformStep], plaintext: &[u8]) -> Result<Vec<u8>> {
    validate_pipeline(steps)?;
    let mut bytes = plaintext.to_vec();
    for step in steps {
        let registration = registration(&step.transform_id)?;
        bytes = (registration.forward)(&step.parameters, &bytes)?;
        if bytes.len() != plaintext.len() {
            return Err(length_mismatch(&step.transform_id));
        }
    }
    Ok(bytes)
}

pub(crate) fn inverse_pipeline(steps: &[TransformStep], encoded: &[u8]) -> Result<Vec<u8>> {
    validate_pipeline(steps)?;
    let mut bytes = encoded.to_vec();
    for step in steps.iter().rev() {
        let registration = registration(&step.transform_id)?;
        bytes = (registration.inverse)(&step.parameters, &bytes)?;
        if bytes.len() != encoded.len() {
            return Err(length_mismatch(&step.transform_id));
        }
    }
    Ok(bytes)
}

pub(crate) fn display_step(step: &TransformStep) -> String {
    if step.transform_id == BYTE_SHUFFLE_ID
        && let [width] = step.parameters.as_ref()
    {
        return format!("byte-shuffle-{width}/v1");
    }
    step.transform_id.clone()
}

fn validate_step(step: &TransformStep) -> Result<()> {
    let registration = registration(&step.transform_id)?;
    (registration.validate)(&step.parameters)
}

fn registration(identifier: &str) -> Result<&'static TransformRegistration> {
    let registration = TRANSFORMS
        .iter()
        .find(|registration| registration.identifier == identifier)
        .ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownTransform,
                format!("required transform '{identifier}' is not registered"),
            )
        })?;
    debug_assert_ne!(registration.format_version, 0);
    debug_assert_eq!(
        registration.reversibility,
        ReversibilityClass::BijectiveAllByteStrings
    );
    Ok(registration)
}

fn validate_delta8(parameters: &[u8]) -> Result<()> {
    if parameters.is_empty() {
        Ok(())
    } else {
        Err(invalid_parameters(
            "delta8/v1 requires an empty parameter value",
        ))
    }
}

fn validate_shuffle(parameters: &[u8]) -> Result<()> {
    if matches!(parameters, [2] | [4] | [8]) {
        Ok(())
    } else {
        Err(invalid_parameters(
            "byte-shuffle/v1 width must be exactly 2, 4, or 8",
        ))
    }
}

fn delta8_forward(_parameters: &[u8], input: &[u8]) -> Result<Vec<u8>> {
    let mut previous = 0_u8;
    Ok(input
        .iter()
        .map(|byte| {
            let delta = byte.wrapping_sub(previous);
            previous = *byte;
            delta
        })
        .collect())
}

fn delta8_inverse(_parameters: &[u8], input: &[u8]) -> Result<Vec<u8>> {
    let mut previous = 0_u8;
    Ok(input
        .iter()
        .map(|delta| {
            previous = previous.wrapping_add(*delta);
            previous
        })
        .collect())
}

fn shuffle_forward(parameters: &[u8], input: &[u8]) -> Result<Vec<u8>> {
    let width = usize::from(parameters[0]);
    let complete = input.len() / width;
    let tail = complete * width;
    let mut output = Vec::with_capacity(input.len());
    for lane in 0..width {
        for item in 0..complete {
            output.push(input[item * width + lane]);
        }
    }
    output.extend_from_slice(&input[tail..]);
    Ok(output)
}

fn shuffle_inverse(parameters: &[u8], input: &[u8]) -> Result<Vec<u8>> {
    let width = usize::from(parameters[0]);
    let complete = input.len() / width;
    let tail = complete * width;
    let mut output = vec![0_u8; tail];
    for lane in 0..width {
        for item in 0..complete {
            output[item * width + lane] = input[lane * complete + item];
        }
    }
    output.extend_from_slice(&input[tail..]);
    Ok(output)
}

fn invalid_parameters(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::InvalidTransformParameters,
        detail,
    )
}

fn length_mismatch(identifier: &str) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::TransformedLengthMismatch,
        format!("transform '{identifier}' did not preserve byte length"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_transforms_are_exactly_invertible_for_all_tail_lengths() {
        for len in 0..67 {
            let input = (0..len)
                .map(|index| (index as u8).wrapping_mul(17))
                .collect::<Vec<_>>();
            for steps in [
                vec![delta8_step()],
                vec![byte_shuffle_step(2).unwrap()],
                vec![byte_shuffle_step(4).unwrap()],
                vec![byte_shuffle_step(8).unwrap()],
                vec![delta8_step(), byte_shuffle_step(4).unwrap()],
            ] {
                let transformed = forward_pipeline(&steps, &input).unwrap();
                assert_eq!(inverse_pipeline(&steps, &transformed).unwrap(), input);
            }
        }
    }

    #[test]
    fn registry_rejects_unknown_duplicate_and_invalid_steps() {
        let unknown = TransformStep {
            transform_id: "unregistered/v1".to_owned(),
            parameters: Box::default(),
        };
        assert_eq!(
            validate_pipeline(&[unknown]).unwrap_err().code(),
            ReasonCode::UnknownTransform
        );
        assert_eq!(
            validate_pipeline(&[delta8_step(), delta8_step()])
                .unwrap_err()
                .code(),
            ReasonCode::DuplicateSemanticDeclaration
        );
        let invalid = TransformStep {
            transform_id: BYTE_SHUFFLE_ID.to_owned(),
            parameters: Box::from([3]),
        };
        assert_eq!(
            validate_pipeline(&[invalid]).unwrap_err().code(),
            ReasonCode::InvalidTransformParameters
        );
    }
}
