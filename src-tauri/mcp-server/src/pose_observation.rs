//! Typed Rust mirror of the published mcp-pose-observation/v1 envelope.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const POSE_OBSERVATION_PROTOCOL: &str = "mcp-pose-observation/v1";
pub const MAX_POSE_REQUEST_BYTES: usize = 256 * 1024;
pub const MIN_POSE_DEADLINE_MS: u32 = 100;
pub const MAX_POSE_DEADLINE_MS: u32 = 30_000;
pub const MAX_POSE_ID_BYTES: usize = 64;
pub const MAX_IMAGE_CHUNK_BYTES: usize = 32 * 1024;
pub const OBSERVATION_IMAGE_REQUEST_EVENT: &str = "m2m-pose-observation-image-request";
pub const OBSERVATION_IMAGE_RESPOND_COMMAND: &str = "mcp_pose_observation_image_respond";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PoseObservationOperation {
    #[serde(rename = "pose.capture")]
    Capture,
    #[serde(rename = "pose.cancel")]
    Cancel,
    #[serde(rename = "pose.dispose")]
    Dispose,
    #[serde(rename = "pose.status")]
    Status,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoseObservationRequest {
    pub protocol: String,
    pub request_id: String,
    pub session_id: String,
    pub sequence: u64,
    pub deadline_ms: u32,
    pub operation: PoseObservationOperation,
    pub payload: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoseObservationError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoseObservationResponse {
    pub protocol: String,
    pub request_id: String,
    pub session_id: String,
    pub sequence: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<PoseObservationError>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoseImageChunkRequest {
    pub request_id: String,
    pub session_id: String,
    pub capture_id: String,
    pub image_ref: String,
    pub view: String,
    pub offset: usize,
    pub max_bytes: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoseImageChunkResponse {
    pub request_id: String,
    pub session_id: String,
    pub capture_id: String,
    pub image_ref: String,
    pub view: String,
    pub offset: usize,
    pub mime: String,
    pub total_bytes: usize,
    pub bytes_base64: String,
}

impl PoseImageChunkRequest {
    pub fn validate(&self) -> Result<(), PoseObservationError> {
        if !valid_id(&self.request_id)
            || !valid_id(&self.session_id)
            || !valid_id(&self.capture_id)
            || self.image_ref != format!("{}:{}", self.capture_id, self.view)
            || !matches!(
                self.view.as_str(),
                "front" | "back" | "left" | "right" | "top" | "bottom"
            )
            || !(1..=MAX_IMAGE_CHUNK_BYTES).contains(&self.max_bytes)
            || self.offset > 1024 * 1024
            || self
                .offset
                .checked_add(self.max_bytes)
                .is_none_or(|end| end > 1024 * 1024)
        {
            return Err(PoseObservationError {
                code: "BAD_REQUEST".into(),
                message: "image chunk request is outside bounds".into(),
            });
        }
        Ok(())
    }

    pub fn validate_response(&self, response: &PoseImageChunkResponse) -> bool {
        let byte_limit = match response.mime.as_str() {
            "image/png" => 1024 * 1024,
            "image/jpeg" => 256 * 1024,
            _ => return false,
        };
        response.request_id == self.request_id
            && response.session_id == self.session_id
            && response.capture_id == self.capture_id
            && response.image_ref == self.image_ref
            && response.view == self.view
            && response.offset == self.offset
            && response.total_bytes > 0
            && response.total_bytes <= byte_limit
            && self.offset < response.total_bytes
            && self
                .offset
                .checked_add(self.max_bytes)
                .is_some_and(|end| end <= response.total_bytes)
            && !response.bytes_base64.is_empty()
            && response.bytes_base64.len() <= self.max_bytes.div_ceil(3) * 4
            && response.bytes_base64.len().is_multiple_of(4)
            && response
                .bytes_base64
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
    }
}

impl PoseObservationRequest {
    pub fn validate(&self) -> Result<(), PoseObservationError> {
        if self.protocol != POSE_OBSERVATION_PROTOCOL {
            return Err(self.error("BAD_PROTOCOL", "unsupported protocol"));
        }
        if self.operation == PoseObservationOperation::Unknown {
            return Err(self.error("UNKNOWN_OPERATION", "operation is not in the v1 allowlist"));
        }
        if !valid_id(&self.request_id)
            || !valid_id(&self.session_id)
            || self.sequence == 0
            || self.sequence > 9_007_199_254_740_991
        {
            return Err(self.error("BAD_REQUEST", "request/session IDs or sequence are invalid"));
        }
        if !(MIN_POSE_DEADLINE_MS..=MAX_POSE_DEADLINE_MS).contains(&self.deadline_ms) {
            return Err(self.error("BAD_REQUEST", "deadline_ms is out of bounds"));
        }
        if !self.payload.is_object() {
            return Err(self.error("BAD_REQUEST", "payload must be a JSON object"));
        }
        if serde_json::to_vec(self)
            .map_err(|_| self.error("BAD_REQUEST", "request is not serializable"))?
            .len()
            > MAX_POSE_REQUEST_BYTES
        {
            return Err(self.error("PAYLOAD_TOO_LARGE", "request exceeds the 256 KiB ceiling"));
        }
        Ok(())
    }

    pub fn error(&self, code: &str, message: &str) -> PoseObservationError {
        PoseObservationError {
            code: code.to_owned(),
            message: message.to_owned(),
        }
    }

    pub fn error_response(&self, error: PoseObservationError) -> PoseObservationResponse {
        PoseObservationResponse {
            protocol: POSE_OBSERVATION_PROTOCOL.to_owned(),
            request_id: self.request_id.clone(),
            session_id: self.session_id.clone(),
            sequence: self.sequence,
            ok: false,
            result: None,
            error: Some(error),
        }
    }

    pub fn validate_response(&self, response: &PoseObservationResponse) -> bool {
        response.protocol == POSE_OBSERVATION_PROTOCOL
            && response.request_id == self.request_id
            && response.session_id == self.session_id
            && response.sequence == self.sequence
            && response.ok == response.result.is_some()
            && response.ok != response.error.is_some()
    }
}

impl PoseObservationResponse {
    pub fn success(request: &PoseObservationRequest, result: Value) -> Self {
        Self {
            protocol: POSE_OBSERVATION_PROTOCOL.to_owned(),
            request_id: request.request_id.clone(),
            session_id: request.session_id.clone(),
            sequence: request.sequence,
            ok: true,
            result: Some(result),
            error: None,
        }
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_POSE_ID_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request() -> PoseObservationRequest {
        PoseObservationRequest {
            protocol: POSE_OBSERVATION_PROTOCOL.into(),
            request_id: "capture-1".into(),
            session_id: "session-1".into(),
            sequence: 1,
            deadline_ms: 30_000,
            operation: PoseObservationOperation::Capture,
            payload: json!({"draft_id":"draft-1", "views":["front"]}),
        }
    }

    #[test]
    fn v1_envelope_round_trips_closed_operations_and_correlation() {
        let request = request();
        request.validate().unwrap();
        let encoded = serde_json::to_string(&request).unwrap();
        assert!(encoded.contains("\"operation\":\"pose.capture\""));
        let response = PoseObservationResponse::success(&request, json!({"capture_id":"opaque"}));
        assert!(request.validate_response(&response));
        let mut mismatched = response;
        mismatched.sequence += 1;
        assert!(!request.validate_response(&mismatched));
    }

    #[test]
    fn rejects_utf8_ids_and_deadline_boundaries() {
        let mut request = request();
        request.session_id = "é".repeat(33);
        assert_eq!(request.validate().unwrap_err().code, "BAD_REQUEST");
        request.session_id = "session".into();
        request.deadline_ms = MAX_POSE_DEADLINE_MS + 1;
        assert_eq!(request.validate().unwrap_err().code, "BAD_REQUEST");
        request.deadline_ms = MIN_POSE_DEADLINE_MS;
        request.validate().unwrap();
        request.sequence = 9_007_199_254_740_992;
        assert_eq!(request.validate().unwrap_err().code, "BAD_REQUEST");
    }

    #[test]
    fn rejects_unknown_operations_and_oversized_utf8_envelopes() {
        let mut request = request();
        request.operation = PoseObservationOperation::Unknown;
        assert_eq!(request.validate().unwrap_err().code, "UNKNOWN_OPERATION");
        request.operation = PoseObservationOperation::Capture;
        request.payload = json!({"padding":"é".repeat(MAX_POSE_REQUEST_BYTES)});
        assert_eq!(request.validate().unwrap_err().code, "PAYLOAD_TOO_LARGE");
    }

    #[test]
    fn image_chunk_requests_are_camera_session_and_byte_bounded() {
        let valid = PoseImageChunkRequest {
            request_id: "chunk-1".into(),
            session_id: "session-1".into(),
            capture_id: "capture-1".into(),
            image_ref: "capture-1:front".into(),
            view: "front".into(),
            offset: 0,
            max_bytes: MAX_IMAGE_CHUNK_BYTES,
        };
        assert!(valid.validate().is_ok());
        let mut invalid = valid.clone();
        invalid.view = "desktop".into();
        assert_eq!(invalid.validate().unwrap_err().code, "BAD_REQUEST");
        let mut invalid = valid.clone();
        invalid.max_bytes = MAX_IMAGE_CHUNK_BYTES + 1;
        assert_eq!(invalid.validate().unwrap_err().code, "BAD_REQUEST");
        let mut invalid = valid;
        invalid.image_ref = "/tmp/capture.png".into();
        assert_eq!(invalid.validate().unwrap_err().code, "BAD_REQUEST");
    }
}
