//! Governed structural and format-aware reconstructive transforms.

use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{Digest, ReconstructionData, TransformStep};
use crate::ecf::{
    FEATURE_CODEC_TRANSFORM_V1, FEATURE_RECONSTRUCTIVE_TRANSFORM_V1,
    FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1,
};
use crate::jpeg_reconstruction::{
    JPEG_RECONSTRUCT_ID, JPEG_RECONSTRUCT_PARAMETERS, inverse as reconstruct_jpeg,
    validate_parameters as validate_jpeg, verified_forward as verified_jpeg_forward,
};
use crate::reconstruction::{
    DEFLATE_RECONSTRUCT_ID, inverse as reconstruct_deflate,
    validate_parameters as validate_deflate, verified_forward as verified_deflate_forward,
};

pub(crate) const DELTA8_ID: &str = "delta8/v1";
pub(crate) const BYTE_SHUFFLE_ID: &str = "byte-shuffle/v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReversibilityClass {
    Structural,
    Reconstructive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReconstructionBacking {
    SideData,
    SelfContained,
}

type StructuralOperation = fn(&[u8], &[u8]) -> Result<Vec<u8>>;

pub(crate) struct TransformRegistration {
    pub identifier: &'static str,
    pub format_version: u16,
    pub required_feature: u64,
    pub reversibility: ReversibilityClass,
    pub reconstruction_backing: Option<ReconstructionBacking>,
    validate: fn(&[u8]) -> Result<()>,
    forward: Option<StructuralOperation>,
    inverse: Option<StructuralOperation>,
}

static TRANSFORMS: [TransformRegistration; 4] = [
    TransformRegistration {
        identifier: DELTA8_ID,
        format_version: 1,
        required_feature: FEATURE_CODEC_TRANSFORM_V1,
        reversibility: ReversibilityClass::Structural,
        reconstruction_backing: None,
        validate: validate_delta8,
        forward: Some(delta8_forward),
        inverse: Some(delta8_inverse),
    },
    TransformRegistration {
        identifier: BYTE_SHUFFLE_ID,
        format_version: 1,
        required_feature: FEATURE_CODEC_TRANSFORM_V1,
        reversibility: ReversibilityClass::Structural,
        reconstruction_backing: None,
        validate: validate_shuffle,
        forward: Some(shuffle_forward),
        inverse: Some(shuffle_inverse),
    },
    TransformRegistration {
        identifier: DEFLATE_RECONSTRUCT_ID,
        format_version: 1,
        required_feature: FEATURE_RECONSTRUCTIVE_TRANSFORM_V1,
        reversibility: ReversibilityClass::Reconstructive,
        reconstruction_backing: Some(ReconstructionBacking::SideData),
        validate: validate_deflate_parameters,
        forward: None,
        inverse: None,
    },
    TransformRegistration {
        identifier: JPEG_RECONSTRUCT_ID,
        format_version: 1,
        required_feature: FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1,
        reversibility: ReversibilityClass::Reconstructive,
        reconstruction_backing: Some(ReconstructionBacking::SelfContained),
        validate: validate_jpeg,
        forward: None,
        inverse: None,
    },
];

pub(crate) fn delta8_step() -> TransformStep {
    TransformStep {
        transform_id: DELTA8_ID.to_owned(),
        parameters: Box::default(),
        reconstruction_ref: None,
    }
}

pub(crate) fn byte_shuffle_step(width: u8) -> Result<TransformStep> {
    let step = TransformStep {
        transform_id: BYTE_SHUFFLE_ID.to_owned(),
        parameters: Box::from([width]),
        reconstruction_ref: None,
    };
    validate_step(&step)?;
    Ok(step)
}

pub(crate) fn deflate_reconstruct_step(
    max_chain_length: u32,
    reconstruction_ref: Digest,
) -> Result<TransformStep> {
    let step = TransformStep {
        transform_id: DEFLATE_RECONSTRUCT_ID.to_owned(),
        parameters: crate::reconstruction::parameters(max_chain_length),
        reconstruction_ref: Some(reconstruction_ref),
    };
    validate_step(&step)?;
    Ok(step)
}

pub(crate) fn jpeg_reconstruct_step() -> Result<TransformStep> {
    let step = TransformStep {
        transform_id: JPEG_RECONSTRUCT_ID.to_owned(),
        parameters: JPEG_RECONSTRUCT_PARAMETERS.into(),
        reconstruction_ref: None,
    };
    validate_step(&step)?;
    Ok(step)
}

pub(crate) fn validate_pipeline(steps: &[TransformStep]) -> Result<()> {
    let mut identifiers = BTreeSet::new();
    let mut reconstructive_count = 0_u8;
    for (position, step) in steps.iter().enumerate() {
        let registration = registration(&step.transform_id)?;
        (registration.validate)(&step.parameters)?;
        match registration.reversibility {
            ReversibilityClass::Structural if step.reconstruction_ref.is_some() => {
                return Err(invalid_parameters(
                    "structural TransformStep cannot reference ReconstructionData",
                ));
            }
            ReversibilityClass::Reconstructive => {
                reconstructive_count = reconstructive_count.saturating_add(1);
                let backing = registration
                    .reconstruction_backing
                    .expect("reconstructive registration declares its backing");
                let reference_valid = match backing {
                    ReconstructionBacking::SideData => step.reconstruction_ref.is_some(),
                    ReconstructionBacking::SelfContained => step.reconstruction_ref.is_none(),
                };
                if position != 0 || !reference_valid {
                    return Err(invalid_parameters(
                        "the sole reconstructive TransformStep must be first and use its registered backing mode",
                    ));
                }
            }
            ReversibilityClass::Structural => {}
        }
        if !identifiers.insert(step.transform_id.as_str()) {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                format!("duplicate TransformStep '{}'", step.transform_id),
            ));
        }
    }
    if reconstructive_count > 1 {
        return Err(invalid_parameters(
            "a TransformPlan may contain at most one reconstructive TransformStep",
        ));
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
    if steps.iter().any(|step| step.reconstruction_ref.is_some()) {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::UnknownReconstructionData,
            "reconstructive pipeline requires its recorded ReconstructionData",
        ));
    }
    forward_pipeline_with_reconstruction(steps, plaintext, &BTreeMap::new())
}

pub(crate) fn forward_pipeline_with_reconstruction(
    steps: &[TransformStep],
    plaintext: &[u8],
    reconstruction_data: &BTreeMap<Digest, ReconstructionData>,
) -> Result<Vec<u8>> {
    validate_pipeline(steps)?;
    let mut bytes = plaintext.to_vec();
    for step in steps {
        let registration = registration(&step.transform_id)?;
        match registration.reversibility {
            ReversibilityClass::Structural => {
                let before = bytes.len();
                bytes =
                    (registration.forward.expect("structural forward"))(&step.parameters, &bytes)?;
                if bytes.len() != before {
                    return Err(length_mismatch(&step.transform_id));
                }
            }
            ReversibilityClass::Reconstructive => match registration.reconstruction_backing {
                Some(ReconstructionBacking::SideData) => {
                    let reference = step.reconstruction_ref.expect("validated reference");
                    let data = reconstruction_data.get(&reference).ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::Nonconforming,
                            ReasonCode::UnknownReconstructionData,
                            reference.to_string(),
                        )
                    })?;
                    bytes = verified_deflate_forward(
                        &bytes,
                        validate_deflate(&step.parameters)?,
                        data,
                    )?;
                }
                Some(ReconstructionBacking::SelfContained) => {
                    bytes = verified_jpeg_forward(&bytes)
                        .map_err(|error| {
                            Diagnostic::new(
                                OutcomeClass::Nonconforming,
                                ReasonCode::TransformFailed,
                                format!("JPEG reconstruction candidate is ineligible: {error:?}"),
                            )
                        })?
                        .bytes;
                }
                None => unreachable!("validated reconstructive registration"),
            },
        }
    }
    Ok(bytes)
}

pub(crate) fn inverse_pipeline(steps: &[TransformStep], encoded: &[u8]) -> Result<Vec<u8>> {
    validate_pipeline(steps)?;
    if steps.iter().any(|step| step.reconstruction_ref.is_some()) {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::UnknownReconstructionData,
            "reconstructive pipeline requires its recorded ReconstructionData",
        ));
    }
    inverse_pipeline_with_reconstruction(steps, encoded, &BTreeMap::new())
}

pub(crate) fn inverse_pipeline_with_reconstruction(
    steps: &[TransformStep],
    encoded: &[u8],
    reconstruction_data: &BTreeMap<Digest, ReconstructionData>,
) -> Result<Vec<u8>> {
    validate_pipeline(steps)?;
    let mut bytes = encoded.to_vec();
    for step in steps.iter().rev() {
        let registration = registration(&step.transform_id)?;
        match registration.reversibility {
            ReversibilityClass::Structural => {
                let before = bytes.len();
                bytes =
                    (registration.inverse.expect("structural inverse"))(&step.parameters, &bytes)?;
                if bytes.len() != before {
                    return Err(length_mismatch(&step.transform_id));
                }
            }
            ReversibilityClass::Reconstructive => match registration.reconstruction_backing {
                Some(ReconstructionBacking::SideData) => {
                    let reference = step.reconstruction_ref.expect("validated reference");
                    let data = reconstruction_data.get(&reference).ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::Nonconforming,
                            ReasonCode::UnknownReconstructionData,
                            reference.to_string(),
                        )
                    })?;
                    bytes = reconstruct_deflate(&bytes, data)?;
                }
                Some(ReconstructionBacking::SelfContained) => {
                    bytes = reconstruct_jpeg(&bytes)?;
                }
                None => unreachable!("validated reconstructive registration"),
            },
        }
    }
    Ok(bytes)
}

pub(crate) fn intermediate_len(
    steps: &[TransformStep],
    logical_len: u64,
    reconstruction_data: &BTreeMap<Digest, ReconstructionData>,
) -> Result<u64> {
    let Some(reference) = steps.first().and_then(|step| step.reconstruction_ref) else {
        return Ok(logical_len);
    };
    reconstruction_data
        .get(&reference)
        .map(|data| data.intermediate_len)
        .ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::UnknownReconstructionData,
                reference.to_string(),
            )
        })
}

pub(crate) fn display_step(step: &TransformStep) -> String {
    if step.transform_id == BYTE_SHUFFLE_ID
        && let [width] = step.parameters.as_ref()
    {
        return format!("byte-shuffle-{width}/v1");
    }
    step.transform_id.clone()
}

pub(crate) fn is_whole_object_reconstructive(step: &TransformStep) -> Result<bool> {
    Ok(registration(&step.transform_id)?.reconstruction_backing
        == Some(ReconstructionBacking::SelfContained))
}

pub(crate) fn is_reconstructive(step: &TransformStep) -> Result<bool> {
    Ok(registration(&step.transform_id)?.reversibility == ReversibilityClass::Reconstructive)
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
    Ok(registration)
}

fn validate_deflate_parameters(parameters: &[u8]) -> Result<()> {
    validate_deflate(parameters).map(|_| ())
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
            reconstruction_ref: None,
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
            reconstruction_ref: None,
        };
        assert_eq!(
            validate_pipeline(&[invalid]).unwrap_err().code(),
            ReasonCode::InvalidTransformParameters
        );
    }

    #[test]
    fn reconstructive_step_order_and_reference_are_canonical() {
        let reference = Digest::from_bytes([7; 32]);
        let reconstruct = deflate_reconstruct_step(512, reference).unwrap();
        validate_pipeline(&[reconstruct.clone(), delta8_step()]).unwrap();
        assert_eq!(
            validate_pipeline(&[delta8_step(), reconstruct.clone()])
                .unwrap_err()
                .code(),
            ReasonCode::InvalidTransformParameters
        );
        assert_eq!(
            validate_pipeline(&[reconstruct.clone(), reconstruct])
                .unwrap_err()
                .code(),
            ReasonCode::InvalidTransformParameters
        );
        let missing = TransformStep {
            transform_id: DEFLATE_RECONSTRUCT_ID.to_owned(),
            parameters: crate::reconstruction::parameters(512),
            reconstruction_ref: None,
        };
        assert_eq!(
            validate_pipeline(&[missing]).unwrap_err().code(),
            ReasonCode::InvalidTransformParameters
        );
        assert_eq!(
            registration(DEFLATE_RECONSTRUCT_ID).unwrap().reversibility,
            ReversibilityClass::Reconstructive
        );
        assert_eq!(
            registration(DELTA8_ID).unwrap().reversibility,
            ReversibilityClass::Structural
        );
    }
}
