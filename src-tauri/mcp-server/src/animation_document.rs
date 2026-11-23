//! Typed mirror and bounded, session-scoped draft storage for AnimationDocument v1.
//! Mutations are exposed only through the validated trusted native frontend
//! request/response bridge.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const ANIMATION_DOCUMENT_SCHEMA: &str = "animation-document/v1";
pub const MAX_DOCUMENT_BYTES: usize = 256 * 1024;
pub const MAX_BONES: usize = 256;
pub const MAX_CHANNELS: usize = 512;
pub const MAX_KEYFRAMES: usize = 4096;
pub const MAX_DRAFTS: usize = 16;
pub const MAX_HISTORY: usize = 32;

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationDocument {
    pub schema_version: String,
    pub rig: AnimationRig,
    pub bones: Vec<AnimationBone>,
    pub clip: AnimationClip,
    pub channels: Vec<AnimationChannel>,
    pub revision: u64,
    pub validation: AnimationValidation,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationRig {
    pub identifier: String,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationBone {
    pub id: String,
    pub display_name: String,
    pub index: u16,
    pub parent_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationClip {
    pub name: String,
    pub duration_seconds: f64,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationChannel {
    pub bone_id: String,
    pub property: AnimationProperty,
    pub keyframes: Vec<AnimationKeyframe>,
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AnimationProperty {
    Position,
    Quaternion,
    Scale,
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Interpolation {
    Linear,
    Step,
    Slerp,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationKeyframe {
    pub time_seconds: f64,
    pub values: Vec<f64>,
    pub interpolation: Interpolation,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub struct AnimationValidation {
    pub state: ValidationState,
    pub errors: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationState {
    Draft,
    Valid,
    Invalid,
}

impl AnimationDocument {
    /// Deterministic validation against the actual catalog supplied by the caller.
    pub fn validate(
        &self,
        fingerprint: &str,
        catalog: &[AnimationBone],
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.schema_version != ANIMATION_DOCUMENT_SCHEMA {
            errors.push("unsupported schema_version".into());
        }
        if self.rig.identifier.is_empty() {
            errors.push("invalid rig identifier".into());
        }
        if self.rig.fingerprint != fingerprint {
            errors.push("rig fingerprint mismatch".into());
        }
        if self.rig.fingerprint.len() != 64
            || !self.rig.fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
        {
            errors.push("fingerprint must be 64 hexadecimal characters".into());
        }
        if self.bones.len() > MAX_BONES {
            errors.push("too many bones".into());
        }
        if self.bones.is_empty() {
            errors.push("bone catalog must not be empty".into());
        }
        if self.bones != catalog {
            errors.push("bone catalog differs from live rig".into());
        }
        let mut bone_ids = std::collections::HashSet::new();
        let mut bone_indices = std::collections::HashSet::new();
        for bone in &self.bones {
            if bone.id.is_empty() || !bone_ids.insert(bone.id.as_str()) {
                errors.push("bone IDs must be non-empty and unique".into());
            }
            if !bone_indices.insert(bone.index) {
                errors.push("bone indices must be unique".into());
            }
            if bone.parent_id.as_deref() == Some(bone.id.as_str()) {
                errors.push(format!("bone {} cannot parent itself", bone.id));
            }
            if bone
                .parent_id
                .as_deref()
                .is_some_and(|parent| !self.bones.iter().any(|candidate| candidate.id == parent))
            {
                errors.push(format!("unknown parent_id for bone {}", bone.id));
            }
        }
        if self.clip.name.trim().is_empty() {
            errors.push("invalid clip name".into());
        }
        if !self.clip.duration_seconds.is_finite()
            || self.clip.duration_seconds <= 0.0
            || self.clip.duration_seconds > 3600.0
        {
            errors.push("duration must satisfy 0 < d <= 3600 seconds".into());
        }
        if self.revision == 0 || self.revision > 9_007_199_254_740_991 {
            errors.push("revision must be positive".into());
        }
        if self.channels.len() > MAX_CHANNELS {
            errors.push("too many channels".into());
        }
        let mut keyframe_count = 0usize;
        let mut channel_keys = std::collections::HashSet::new();
        let bone_ids: std::collections::HashSet<&str> =
            self.bones.iter().map(|b| b.id.as_str()).collect();
        for channel in &self.channels {
            if !bone_ids.contains(channel.bone_id.as_str()) {
                errors.push(format!("unknown bone_id {}", channel.bone_id));
            }
            if !channel_keys.insert((channel.bone_id.as_str(), channel.property as u8)) {
                errors.push("duplicate channel".into());
            }
            if channel.keyframes.is_empty() {
                errors.push("channel must contain keyframes".into());
            }
            let mut previous = f64::NEG_INFINITY;
            for frame in &channel.keyframes {
                keyframe_count = keyframe_count.saturating_add(1);
                if !frame.time_seconds.is_finite() {
                    errors.push("keyframe time must be finite".into());
                } else {
                    if frame.time_seconds < 0.0 {
                        errors.push("keyframe time must be non-negative".into());
                    }
                    if frame.time_seconds <= previous {
                        errors.push("keyframe times must be strictly increasing".into());
                    }
                    if frame.time_seconds > self.clip.duration_seconds {
                        errors.push("keyframe time exceeds clip duration".into());
                    }
                    previous = frame.time_seconds;
                }
                let expected = if channel.property == AnimationProperty::Quaternion {
                    4
                } else {
                    3
                };
                if frame.values.len() != expected
                    || frame.values.iter().any(|value| !value.is_finite())
                {
                    errors.push("invalid or non-finite keyframe values".into());
                }
                if channel.property == AnimationProperty::Quaternion && frame.values.len() == 4 {
                    let norm = frame.values.iter().map(|v| v * v).sum::<f64>().sqrt();
                    if !norm.is_finite() || (norm - 1.0).abs() > 1e-4 {
                        errors.push("quaternion must be normalized XYZW".into());
                    }
                    if frame.interpolation != Interpolation::Slerp
                        && frame.interpolation != Interpolation::Step
                    {
                        errors.push("quaternion interpolation must be slerp or step".into());
                    }
                } else if frame.interpolation == Interpolation::Slerp {
                    errors.push("slerp is only valid for quaternion channels".into());
                }
            }
        }
        if keyframe_count > MAX_KEYFRAMES {
            errors.push("too many keyframes".into());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DraftError {
    TooLarge,
    Invalid(Vec<String>),
    Missing,
    RevisionConflict,
    Capacity,
    LockPoisoned,
}

/// Identity and catalog are read directly from the live application skeleton.
pub struct LiveRigCatalog<'a> {
    pub fingerprint: &'a str,
    pub bones: &'a [AnimationBone],
}

#[derive(Clone)]
struct Draft {
    current: AnimationDocument,
    history: Vec<AnimationDocument>,
    bytes: usize,
}

/// Limits active session drafts and per-draft history; only validated documents enter.
#[derive(Clone, Default)]
pub struct AnimationDraftStore(Arc<Mutex<BTreeMap<String, Draft>>>);

impl AnimationDraftStore {
    pub fn create(
        &self,
        session: &str,
        mut document: AnimationDocument,
        fingerprint: &str,
        catalog: &[AnimationBone],
    ) -> Result<(), DraftError> {
        if session.is_empty() || session.len() > 128 {
            return Err(DraftError::Invalid(vec![
                "invalid session identifier".into()
            ]));
        }
        let bytes = serde_json::to_vec(&document)
            .map_err(|_| DraftError::TooLarge)?
            .len();
        if bytes > MAX_DOCUMENT_BYTES {
            return Err(DraftError::TooLarge);
        }
        document
            .validate(fingerprint, catalog)
            .map_err(DraftError::Invalid)?;
        document.validation = AnimationValidation {
            state: ValidationState::Valid,
            errors: Vec::new(),
        };
        let mut entries = self.0.lock().map_err(|_| DraftError::LockPoisoned)?;
        if entries.contains_key(session) {
            return Err(DraftError::RevisionConflict);
        }
        if entries.len() >= MAX_DRAFTS {
            return Err(DraftError::Capacity);
        }
        entries.insert(
            session.to_owned(),
            Draft {
                current: document.clone(),
                history: vec![document],
                bytes,
            },
        );
        Ok(())
    }

    pub fn get(&self, session: &str) -> Result<AnimationDocument, DraftError> {
        self.0
            .lock()
            .map_err(|_| DraftError::LockPoisoned)?
            .get(session)
            .map(|d| d.current.clone())
            .ok_or(DraftError::Missing)
    }

    /// Atomically replace one channel's keyframes and append a new revision.
    /// This is the bounded primitive used by a future incremental MCP operation;
    /// it deliberately accepts only a live catalog supplied by the application.
    pub fn set_bone_keyframes(
        &self,
        session: &str,
        expected_revision: u64,
        replacement: AnimationChannel,
        live_rig: LiveRigCatalog<'_>,
    ) -> Result<AnimationDocument, DraftError> {
        if replacement.keyframes.len() > MAX_KEYFRAMES {
            return Err(DraftError::Invalid(vec!["too many keyframes".into()]));
        }
        let encoded = serde_json::to_vec(&replacement).map_err(|_| DraftError::TooLarge)?;
        if encoded.len() > MAX_DOCUMENT_BYTES {
            return Err(DraftError::TooLarge);
        }

        let mut next = self.get(session)?;
        if next.revision != expected_revision {
            return Err(DraftError::RevisionConflict);
        }
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or(DraftError::RevisionConflict)?;
        let channel = next.channels.iter_mut().find(|channel| {
            channel.bone_id == replacement.bone_id && channel.property == replacement.property
        });
        if let Some(channel) = channel {
            channel.keyframes = replacement.keyframes;
        } else {
            next.channels.push(replacement);
        }
        next.revision = next_revision;
        self.commit(
            session,
            expected_revision,
            next,
            live_rig.fingerprint,
            live_rig.bones,
        )
    }

    pub fn commit(
        &self,
        session: &str,
        expected_revision: u64,
        mut next: AnimationDocument,
        fingerprint: &str,
        catalog: &[AnimationBone],
    ) -> Result<AnimationDocument, DraftError> {
        let bytes = serde_json::to_vec(&next)
            .map_err(|_| DraftError::TooLarge)?
            .len();
        if bytes > MAX_DOCUMENT_BYTES {
            return Err(DraftError::TooLarge);
        }
        next.validate(fingerprint, catalog)
            .map_err(DraftError::Invalid)?;
        next.validation = AnimationValidation {
            state: ValidationState::Valid,
            errors: Vec::new(),
        };
        let mut entries = self.0.lock().map_err(|_| DraftError::LockPoisoned)?;
        let draft = entries.get_mut(session).ok_or(DraftError::Missing)?;
        if draft.current.revision != expected_revision
            || Some(next.revision) != expected_revision.checked_add(1)
        {
            return Err(DraftError::RevisionConflict);
        }
        draft.current = next.clone();
        draft.bytes = bytes;
        draft.history.push(next.clone());
        if draft.history.len() > MAX_HISTORY {
            draft.history.remove(0);
        }
        Ok(next)
    }

    /// Restore a retained snapshot as a new revision without rewriting history.
    pub fn undo(
        &self,
        session: &str,
        expected_revision: u64,
        target_revision: u64,
        fingerprint: &str,
        catalog: &[AnimationBone],
    ) -> Result<AnimationDocument, DraftError> {
        let mut entries = self.0.lock().map_err(|_| DraftError::LockPoisoned)?;
        let draft = entries.get_mut(session).ok_or(DraftError::Missing)?;
        if draft.current.revision != expected_revision {
            return Err(DraftError::RevisionConflict);
        }
        let Some(mut restored) = draft
            .history
            .iter()
            .find(|document| document.revision == target_revision)
            .cloned()
        else {
            return Err(DraftError::Missing);
        };
        restored.revision = expected_revision
            .checked_add(1)
            .ok_or(DraftError::RevisionConflict)?;
        restored
            .validate(fingerprint, catalog)
            .map_err(DraftError::Invalid)?;
        restored.validation = AnimationValidation {
            state: ValidationState::Valid,
            errors: Vec::new(),
        };
        let bytes = serde_json::to_vec(&restored)
            .map_err(|_| DraftError::TooLarge)?
            .len();
        if bytes > MAX_DOCUMENT_BYTES {
            return Err(DraftError::TooLarge);
        }
        draft.bytes = bytes;
        draft.current = restored.clone();
        draft.history.push(restored.clone());
        if draft.history.len() > MAX_HISTORY {
            draft.history.remove(0);
        }
        Ok(restored)
    }

    pub fn remove(&self, session: &str) -> Result<(), DraftError> {
        self.0
            .lock()
            .map_err(|_| DraftError::LockPoisoned)?
            .remove(session)
            .map(|_| ())
            .ok_or(DraftError::Missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> (AnimationDocument, Vec<AnimationBone>) {
        let bones = vec![AnimationBone {
            id: "human:bone:000".into(),
            display_name: "root".into(),
            index: 0,
            parent_id: None,
        }];
        let doc = AnimationDocument {
            schema_version: ANIMATION_DOCUMENT_SCHEMA.into(),
            rig: AnimationRig {
                identifier: "human".into(),
                fingerprint: "a".repeat(64),
            },
            bones: bones.clone(),
            clip: AnimationClip {
                name: "walk".into(),
                duration_seconds: 1.0,
            },
            channels: vec![AnimationChannel {
                bone_id: bones[0].id.clone(),
                property: AnimationProperty::Quaternion,
                keyframes: vec![AnimationKeyframe {
                    time_seconds: 0.0,
                    values: vec![0.0, 0.0, 0.0, 1.0],
                    interpolation: Interpolation::Slerp,
                }],
            }],
            revision: 1,
            validation: AnimationValidation {
                state: ValidationState::Valid,
                errors: vec![],
            },
        };
        (doc, bones)
    }
    #[test]
    fn accepts_contract_document() {
        let (doc, bones) = valid();
        assert!(doc.validate(&"a".repeat(64), &bones).is_ok());
    }
    #[test]
    fn rejects_schema_fingerprint_reference_duration_and_nonfinite() {
        let (mut doc, bones) = valid();
        doc.schema_version = "wrong".into();
        doc.rig.fingerprint = "b".repeat(64);
        doc.channels[0].bone_id = "unknown".into();
        doc.clip.duration_seconds = f64::INFINITY;
        assert_eq!(doc.validate(&"a".repeat(64), &bones).unwrap_err().len(), 4);
    }
    #[test]
    fn rejects_bad_order_shape_quaternion_and_limits() {
        let (mut doc, bones) = valid();
        doc.channels[0].keyframes[0].values = vec![0.0, 0.0, 0.0, 2.0];
        doc.channels[0].keyframes.push(AnimationKeyframe {
            time_seconds: 0.0,
            values: vec![0.0, 0.0, 0.0, 1.0],
            interpolation: Interpolation::Slerp,
        });
        assert!(doc.validate(&"a".repeat(64), &bones).unwrap_err().len() >= 2);
    }

    #[test]
    fn rejects_negative_first_keyframe_time() {
        let (mut doc, bones) = valid();
        doc.channels[0].keyframes[0].time_seconds = -0.1;
        assert!(doc
            .validate(&"a".repeat(64), &bones)
            .unwrap_err()
            .iter()
            .any(|error| error.contains("non-negative")));
    }

    #[test]
    fn rejects_malformed_catalog_and_empty_channels() {
        let (mut doc, mut bones) = valid();
        bones[0].parent_id = Some(bones[0].id.clone());
        doc.bones = bones.clone();
        doc.channels[0].keyframes.clear();
        let errors = doc.validate(&"a".repeat(64), &bones).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.contains("cannot parent itself")));
        assert!(errors
            .iter()
            .any(|error| error.contains("channel must contain keyframes")));
    }
    #[test]
    fn mutation_is_revision_checked_and_atomic() {
        let (doc, bones) = valid();
        let store = AnimationDraftStore::default();
        let fp = "a".repeat(64);
        store.create("client-1", doc.clone(), &fp, &bones).unwrap();
        let mut next = doc;
        next.revision = 2;
        next.clip.name = "run".into();
        assert!(store
            .commit("client-1", 1, next.clone(), &fp, &bones)
            .is_ok());
        let mut stale = next.clone();
        stale.revision = 3;
        assert_eq!(
            store.commit("client-1", 1, stale, &fp, &bones),
            Err(DraftError::RevisionConflict)
        );
        assert_eq!(store.get("client-1").unwrap(), next);
    }

    #[test]
    fn keyframe_replacement_appends_revision_atomically() {
        let (doc, bones) = valid();
        let store = AnimationDraftStore::default();
        let fingerprint = "a".repeat(64);
        store
            .create("client-1", doc.clone(), &fingerprint, &bones)
            .unwrap();

        let replacement = vec![
            AnimationKeyframe {
                time_seconds: 0.0,
                values: vec![0.0, 0.0, 0.0, 1.0],
                interpolation: Interpolation::Slerp,
            },
            AnimationKeyframe {
                time_seconds: 0.5,
                values: vec![
                    0.0,
                    0.0,
                    std::f64::consts::FRAC_1_SQRT_2,
                    std::f64::consts::FRAC_1_SQRT_2,
                ],
                interpolation: Interpolation::Slerp,
            },
        ];
        let committed = store
            .set_bone_keyframes(
                "client-1",
                1,
                AnimationChannel {
                    bone_id: bones[0].id.clone(),
                    property: AnimationProperty::Quaternion,
                    keyframes: replacement.clone(),
                },
                LiveRigCatalog {
                    fingerprint: &fingerprint,
                    bones: &bones,
                },
            )
            .unwrap();
        assert_eq!(committed.revision, 2);
        assert_eq!(committed.channels[0].keyframes, replacement);

        let before_rejection = store.get("client-1").unwrap();
        let invalid = store.set_bone_keyframes(
            "client-1",
            2,
            AnimationChannel {
                bone_id: "human:bone:999".into(),
                property: AnimationProperty::Position,
                keyframes: vec![AnimationKeyframe {
                    time_seconds: 0.0,
                    values: vec![0.0, 0.0, 0.0],
                    interpolation: Interpolation::Linear,
                }],
            },
            LiveRigCatalog {
                fingerprint: &fingerprint,
                bones: &bones,
            },
        );
        assert!(matches!(invalid, Err(DraftError::Invalid(_))));
        assert_eq!(store.get("client-1").unwrap(), before_rejection);

        let stale = store.set_bone_keyframes(
            "client-1",
            1,
            AnimationChannel {
                bone_id: bones[0].id.clone(),
                property: AnimationProperty::Quaternion,
                keyframes: replacement,
            },
            LiveRigCatalog {
                fingerprint: &fingerprint,
                bones: &bones,
            },
        );
        assert_eq!(stale, Err(DraftError::RevisionConflict));
        assert_eq!(store.get("client-1").unwrap(), before_rejection);
    }

    #[test]
    fn keyframe_replacement_rejects_payload_over_limits_without_mutation() {
        let (doc, bones) = valid();
        let store = AnimationDraftStore::default();
        let fingerprint = "a".repeat(64);
        store
            .create("client-1", doc.clone(), &fingerprint, &bones)
            .unwrap();
        let too_many = vec![
            AnimationKeyframe {
                time_seconds: 0.0,
                values: vec![0.0, 0.0, 0.0, 1.0],
                interpolation: Interpolation::Slerp,
            };
            MAX_KEYFRAMES + 1
        ];
        assert!(matches!(
            store.set_bone_keyframes(
                "client-1",
                1,
                AnimationChannel {
                    bone_id: bones[0].id.clone(),
                    property: AnimationProperty::Quaternion,
                    keyframes: too_many,
                },
                LiveRigCatalog {
                    fingerprint: &fingerprint,
                    bones: &bones,
                },
            ),
            Err(DraftError::Invalid(_))
        ));
        assert_eq!(store.get("client-1").unwrap(), doc);
    }

    #[test]
    fn undo_restores_snapshot_as_new_revision() {
        let (doc, bones) = valid();
        let store = AnimationDraftStore::default();
        let fp = "a".repeat(64);
        store.create("client-1", doc.clone(), &fp, &bones).unwrap();
        let mut next = doc.clone();
        next.revision = 2;
        next.clip.name = "run".into();
        store.commit("client-1", 1, next, &fp, &bones).unwrap();

        let restored = store.undo("client-1", 2, 1, &fp, &bones).unwrap();
        assert_eq!(restored.revision, 3);
        assert_eq!(restored.clip.name, "walk");
        assert_eq!(store.get("client-1").unwrap(), restored);
        assert_eq!(
            store.get("client-1").unwrap().validation.state,
            ValidationState::Valid
        );
    }
}
