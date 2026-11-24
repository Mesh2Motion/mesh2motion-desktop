use std::sync::{Arc, RwLock};

use crate::ControlSnapshot;

pub const MAX_SNAPSHOT_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreStatus {
    Unavailable,
    Ready,
    IncompatibleVersion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreError {
    TooLarge,
    InvalidSnapshot,
    IncompatibleVersion,
    NonMonotonicSequence,
    Unavailable,
}

/// Bounded diagnostics for the most recent publisher lifecycle. Contains no
/// snapshot payload, asset name, or private path.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StoreDiagnostics {
    pub publish_attempts: u64,
    pub publish_accepted: u64,
    pub publish_rejected: u64,
    pub last_rejection: Option<StoreError>,
    pub snapshot_available: bool,
    pub last_accepted_sequence: Option<u64>,
    pub last_accepted_at: Option<String>,
    pub snapshot_age_ms: Option<u64>,
}

#[derive(Default)]
struct State {
    snapshot: Option<ControlSnapshot>,
    incompatible: bool,
    publish_attempts: u64,
    publish_accepted: u64,
    publish_rejected: u64,
    last_rejection: Option<StoreError>,
}

#[derive(Clone, Default)]
pub struct SnapshotStore(Arc<RwLock<State>>);

impl SnapshotStore {
    pub fn publish_json(&self, value: serde_json::Value) -> Result<(), StoreError> {
        if let Ok(mut state) = self.0.write() {
            state.publish_attempts = state.publish_attempts.saturating_add(1);
        }
        let result = self.publish_json_inner(value);
        if let Ok(mut state) = self.0.write() {
            match result {
                Ok(()) => {
                    state.publish_accepted = state.publish_accepted.saturating_add(1);
                    state.last_rejection = None;
                }
                Err(error) => {
                    state.publish_rejected = state.publish_rejected.saturating_add(1);
                    state.last_rejection = Some(error);
                }
            }
        }
        result
    }

    fn publish_json_inner(&self, value: serde_json::Value) -> Result<(), StoreError> {
        let size = serde_json::to_vec(&value)
            .map_err(|_| StoreError::InvalidSnapshot)?
            .len();
        if size > MAX_SNAPSHOT_BYTES {
            return Err(StoreError::TooLarge);
        }
        log_fractional_number_paths(&value, "", &mut 0);
        let schema = value
            .get("meta")
            .and_then(|m| m.get("schema_version").or_else(|| m.get("schemaVersion")));
        match schema.and_then(serde_json::Value::as_str) {
            Some("1.0") => {}
            Some(_) => {
                if let Ok(mut state) = self.0.write() {
                    state.incompatible = true;
                }
                return Err(StoreError::IncompatibleVersion);
            }
            None => return Err(StoreError::InvalidSnapshot),
        }
        let mut snapshot: ControlSnapshot = serde_json::from_value(value).map_err(|error| {
            eprintln!(
                "M2M_SNAPSHOT_VALIDATION stage=deserialize category={:?} detail={}",
                error.classify(),
                error
            );
            StoreError::InvalidSnapshot
        })?;
        snapshot.validate().map_err(|e| {
            eprintln!("M2M_SNAPSHOT_VALIDATION stage=contract reason={e}");
            if e.contains("incompatible") {
                StoreError::IncompatibleVersion
            } else {
                StoreError::InvalidSnapshot
            }
        })?;
        snapshot.refresh_age();
        let mut state = self.0.write().map_err(|_| StoreError::Unavailable)?;
        if state
            .snapshot
            .as_ref()
            .is_some_and(|old| snapshot.meta.sequence <= old.meta.sequence)
        {
            return Err(StoreError::NonMonotonicSequence);
        }
        state.incompatible = false;
        state.snapshot = Some(snapshot);
        Ok(())
    }

    pub fn diagnostics(&self) -> StoreDiagnostics {
        let Ok(state) = self.0.read() else {
            return StoreDiagnostics {
                last_rejection: Some(StoreError::Unavailable),
                ..StoreDiagnostics::default()
            };
        };
        let mut snapshot = state.snapshot.clone();
        if let Some(snapshot) = snapshot.as_mut() {
            snapshot.refresh_age();
        }
        StoreDiagnostics {
            publish_attempts: state.publish_attempts,
            publish_accepted: state.publish_accepted,
            publish_rejected: state.publish_rejected,
            last_rejection: state.last_rejection,
            snapshot_available: snapshot.is_some(),
            last_accepted_sequence: snapshot.as_ref().map(|s| s.meta.sequence),
            last_accepted_at: snapshot.as_ref().map(|s| s.meta.captured_at.clone()),
            snapshot_age_ms: snapshot.map(|s| s.meta.age_ms),
        }
    }

    pub fn status(&self) -> StoreStatus {
        match self.0.read() {
            Ok(state) if state.incompatible => StoreStatus::IncompatibleVersion,
            Ok(state) if state.snapshot.is_some() => StoreStatus::Ready,
            _ => StoreStatus::Unavailable,
        }
    }

    pub fn snapshot(&self) -> Result<ControlSnapshot, StoreStatus> {
        let state = self.0.read().map_err(|_| StoreStatus::Unavailable)?;
        if state.incompatible {
            return Err(StoreStatus::IncompatibleVersion);
        }
        let mut snapshot = state.snapshot.clone().ok_or(StoreStatus::Unavailable)?;
        snapshot.refresh_age();
        Ok(snapshot)
    }

    #[cfg(test)]
    pub fn poison_for_test(&self) {
        let store = self.clone();
        let _ = std::thread::spawn(move || {
            let _guard = store.0.write().unwrap();
            panic!("intentional poison test");
        })
        .join();
    }
}

fn log_fractional_number_paths(value: &serde_json::Value, path: &str, count: &mut usize) {
    if *count >= 64 {
        return;
    }
    match value {
        serde_json::Value::Object(fields) => {
            for (key, child) in fields {
                let next = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                log_fractional_number_paths(child, &next, count);
                if *count >= 64 {
                    break;
                }
            }
        }
        serde_json::Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                log_fractional_number_paths(child, &format!("{path}[{index}]"), count);
                if *count >= 64 {
                    break;
                }
            }
        }
        serde_json::Value::Number(number) => {
            const INTEGER_KEYS: &[&str] = &[
                "sequence",
                "age_ms",
                "ageMs",
                "uptime_ms",
                "uptimeMs",
                "width",
                "height",
                "consecutive_failures",
                "consecutiveFailures",
                "index",
                "track_count",
                "trackCount",
                "mesh_count",
                "meshCount",
                "vertex_count",
                "vertexCount",
                "triangle_count",
                "triangleCount",
                "bone_count",
                "boneCount",
                "loaded_count",
                "loadedCount",
                "active_index",
                "activeIndex",
                "object_count",
                "objectCount",
            ];
            let key = path.rsplit(['.', '[']).next().unwrap_or(path);
            if INTEGER_KEYS.contains(&key)
                && number.as_f64().is_some_and(|value| value.fract() != 0.0)
            {
                eprintln!("M2M_SNAPSHOT_NUMBER path={path} fractional=true");
                *count += 1;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::mock_snapshot;

    fn json(sequence: u64) -> serde_json::Value {
        let mut snapshot = mock_snapshot();
        snapshot.meta.sequence = sequence;
        serde_json::to_value(snapshot).unwrap()
    }

    fn frontend_keys(value: &mut serde_json::Value) {
        const KEYS: &[(&str, &str)] = &[
            ("schemaVersion", "schema_version"),
            ("capturedAt", "captured_at"),
            ("ageMs", "age_ms"),
            ("uptimeMs", "uptime_ms"),
            ("diagnosticsEnabled", "diagnostics_enabled"),
            ("lastError", "last_error"),
            ("contextLost", "context_lost"),
            ("loopState", "loop"),
            ("consecutiveFailures", "consecutive_failures"),
            ("displayName", "display_name"),
            ("modelAsset", "model_asset"),
            ("rigAsset", "rig_asset"),
            ("animationAssets", "animation_assets"),
            ("durationS", "duration_s"),
            ("trackCount", "track_count"),
            ("sourceType", "source_type"),
            ("meshCount", "mesh_count"),
            ("vertexCount", "vertex_count"),
            ("triangleCount", "triangle_count"),
            ("skeletonType", "type"),
            ("boneCount", "bone_count"),
            ("boneNames", "bone_names"),
            ("rootName", "root_name"),
            ("loadedCount", "loaded_count"),
            ("activeIndex", "active_index"),
            ("processStep", "process_step"),
            ("objectCount", "object_count"),
        ];
        match value {
            serde_json::Value::Object(object) => {
                for (_, value) in object.iter_mut() {
                    frontend_keys(value);
                }
                for (camel, snake) in KEYS {
                    if let Some(value) = object.remove(*camel) {
                        object.insert((*snake).to_owned(), value);
                    }
                }
            }
            serde_json::Value::Array(array) => {
                for value in array {
                    frontend_keys(value);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn accepts_frontend_v1_shape_and_enforces_increasing_sequence() {
        let store = SnapshotStore::default();
        assert_eq!(store.publish_json(json(1)), Ok(()));
        assert_eq!(
            store.publish_json(json(1)),
            Err(StoreError::NonMonotonicSequence)
        );
        assert_eq!(store.publish_json(json(2)), Ok(()));
    }

    #[test]
    fn accepts_pinned_frontend_v1_snake_case_contract() {
        let store = SnapshotStore::default();
        let mut frontend = json(1);
        frontend_keys(&mut frontend);
        let parsed: Result<crate::ControlSnapshot, _> = serde_json::from_value(frontend.clone());
        assert!(
            parsed.is_ok(),
            "frontend snapshot parse failed: {:?}",
            parsed.err()
        );
        assert_eq!(store.publish_json(frontend), Ok(()));
        assert_eq!(
            store
                .snapshot()
                .unwrap()
                .renderer
                .loop_state
                .consecutive_failures,
            0
        );
    }

    #[test]
    fn rejects_malformed_oversized_and_incompatible_snapshots() {
        let store = SnapshotStore::default();
        assert_eq!(
            store.publish_json(serde_json::json!({"bad":true})),
            Err(StoreError::InvalidSnapshot)
        );
        let mut incompatible = json(2);
        incompatible["meta"]["schema_version"] = serde_json::Value::String("2.0".into());
        assert_eq!(
            store.publish_json(incompatible),
            Err(StoreError::IncompatibleVersion)
        );
        assert_eq!(store.status(), StoreStatus::IncompatibleVersion);
        let mut oversized = json(2);
        oversized["padding"] = serde_json::Value::String("x".repeat(MAX_SNAPSHOT_BYTES));
        assert_eq!(store.publish_json(oversized), Err(StoreError::TooLarge));
        assert_eq!(
            store.publish_json(serde_json::json!({"meta":{"schema_version":"1.0"}})),
            Err(StoreError::InvalidSnapshot)
        );
    }

    #[test]
    fn missing_and_poisoned_state_fail_closed() {
        let store = SnapshotStore::default();
        assert_eq!(store.snapshot().unwrap_err(), StoreStatus::Unavailable);
        store.poison_for_test();
        assert_eq!(store.status(), StoreStatus::Unavailable);
        assert_eq!(store.snapshot().unwrap_err(), StoreStatus::Unavailable);
    }

    #[test]
    fn concurrent_publishers_keep_the_highest_sequence() {
        let store = SnapshotStore::default();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(12));
        let workers: Vec<_> = (1..=12)
            .map(|sequence| {
                let store = store.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    let _ = store.publish_json(json(sequence));
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(store.snapshot().unwrap().meta.sequence, 12);
    }

    #[test]
    fn diagnostics_distinguish_attempts_accepted_and_rejected_state() {
        let store = SnapshotStore::default();
        assert_eq!(store.publish_json(json(1)), Ok(()));
        assert_eq!(
            store.publish_json(json(1)),
            Err(StoreError::NonMonotonicSequence)
        );
        let diagnostics = store.diagnostics();
        assert_eq!(diagnostics.publish_attempts, 2);
        assert_eq!(diagnostics.publish_accepted, 1);
        assert_eq!(diagnostics.publish_rejected, 1);
        assert_eq!(
            diagnostics.last_rejection,
            Some(StoreError::NonMonotonicSequence)
        );
        assert_eq!(diagnostics.last_accepted_sequence, Some(1));
        assert!(diagnostics.snapshot_age_ms.is_some());
    }
}
