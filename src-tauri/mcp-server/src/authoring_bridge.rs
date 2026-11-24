//! Typed Rust envelope for the reviewed authoring-bridge/v1 frontend protocol.
//! The IPC frame limit remains 256 KiB even though the initial provisional
//! TypeScript draft allowed 512 KiB.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const AUTHORING_BRIDGE_PROTOCOL: &str = "authoring-bridge/v1";
pub const MAX_AUTHORING_REQUEST_BYTES: usize = 256 * 1024;
pub const MIN_AUTHORING_DEADLINE_MS: u32 = 100;
pub const MAX_AUTHORING_DEADLINE_MS: u32 = 30_000;
pub const MAX_AUTHORING_ID_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AuthoringOperation {
    #[serde(rename = "rig.catalog.get")]
    RigCatalogGet,
    #[serde(rename = "draft.create")]
    DraftCreate,
    #[serde(rename = "draft.set_keyframes")]
    DraftSetKeyframes,
    #[serde(rename = "draft.validate")]
    DraftValidate,
    #[serde(rename = "draft.preview.start")]
    DraftPreviewStart,
    #[serde(rename = "draft.preview.stop")]
    DraftPreviewStop,
    #[serde(rename = "draft.commit")]
    DraftCommit,
    #[serde(rename = "draft.cancel")]
    DraftCancel,
    #[serde(rename = "draft.undo")]
    DraftUndo,
    #[serde(rename = "draft.export")]
    DraftExport,
    #[serde(rename = "draft.import")]
    DraftImport,
    #[serde(rename = "session.status")]
    SessionStatus,
    #[serde(rename = "session.cancel")]
    SessionCancel,
    #[serde(rename = "application.set_page")]
    ApplicationSetPage,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoringRequest {
    pub protocol: String,
    pub request_id: String,
    pub session_id: String,
    /// Strictly increasing within a session; zero is reserved as invalid.
    pub sequence: u64,
    pub deadline_ms: u32,
    pub operation: AuthoringOperation,
    pub payload: Value,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoringError {
    pub code: AuthoringErrorCode,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthoringErrorCode {
    BadProtocol,
    UnknownOperation,
    BadRequest,
    PayloadTooLarge,
    DeadlineExceeded,
    DuplicateRequestId,
    SessionConflict,
    RigUnavailable,
    RigMismatch,
    InvalidDocument,
    PreviewActive,
    ConfirmationRequired,
    ConfirmationDenied,
    ConfirmationExpired,
    CommitFailed,
    UndoFailed,
    Internal,
    AuthoringDisabled,
    AppDisconnected,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoringResponse {
    pub protocol: String,
    pub request_id: String,
    pub session_id: String,
    pub sequence: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AuthoringError>,
}

impl AuthoringRequest {
    pub fn validate(&self) -> Result<(), AuthoringError> {
        if self.protocol != AUTHORING_BRIDGE_PROTOCOL {
            return Err(self.error(AuthoringErrorCode::BadProtocol, "unsupported protocol"));
        }
        if self.operation == AuthoringOperation::Unknown {
            return Err(self.error(
                AuthoringErrorCode::UnknownOperation,
                "operation is not in the authoring-bridge/v1 allowlist",
            ));
        }
        if !valid_id(&self.request_id) || !valid_id(&self.session_id) || self.sequence == 0 {
            return Err(self.error(
                AuthoringErrorCode::BadRequest,
                "request/session IDs and sequence are invalid",
            ));
        }
        if !(MIN_AUTHORING_DEADLINE_MS..=MAX_AUTHORING_DEADLINE_MS).contains(&self.deadline_ms) {
            return Err(self.error(
                AuthoringErrorCode::BadRequest,
                "deadline_ms is out of bounds",
            ));
        }
        if !self.payload.is_object() {
            return Err(self.error(
                AuthoringErrorCode::BadRequest,
                "payload must be a JSON object",
            ));
        }
        if serde_json::to_vec(self)
            .map_err(|_| self.error(AuthoringErrorCode::BadRequest, "invalid payload"))?
            .len()
            > MAX_AUTHORING_REQUEST_BYTES
        {
            return Err(self.error(
                AuthoringErrorCode::PayloadTooLarge,
                "request exceeds the 256 KiB IPC ceiling",
            ));
        }
        Ok(())
    }

    pub fn error(&self, code: AuthoringErrorCode, message: &str) -> AuthoringError {
        AuthoringError {
            code,
            message: message.to_owned(),
        }
    }

    pub fn error_response(&self, error: AuthoringError) -> AuthoringResponse {
        AuthoringResponse {
            protocol: AUTHORING_BRIDGE_PROTOCOL.to_owned(),
            request_id: self.request_id.clone(),
            session_id: self.session_id.clone(),
            sequence: self.sequence,
            ok: false,
            result: None,
            error: Some(error),
        }
    }

    pub fn success_response(&self, result: Value) -> AuthoringResponse {
        AuthoringResponse {
            protocol: AUTHORING_BRIDGE_PROTOCOL.to_owned(),
            request_id: self.request_id.clone(),
            session_id: self.session_id.clone(),
            sequence: self.sequence,
            ok: true,
            result: Some(result),
            error: None,
        }
    }
}

impl AuthoringResponse {
    pub fn validate_for(&self, request: &AuthoringRequest) -> bool {
        self.protocol == AUTHORING_BRIDGE_PROTOCOL
            && self.request_id == request.request_id
            && self.session_id == request.session_id
            && self.sequence == request.sequence
            && self.ok == self.result.is_some()
            && self.ok != self.error.is_some()
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_AUTHORING_ID_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request() -> AuthoringRequest {
        AuthoringRequest {
            protocol: AUTHORING_BRIDGE_PROTOCOL.into(),
            request_id: "request-1".into(),
            session_id: "session-1".into(),
            sequence: 1,
            deadline_ms: 30_000,
            operation: AuthoringOperation::DraftCreate,
            payload: json!({"document": {"schema_version": "animation-document/v1"}}),
        }
    }

    #[test]
    fn operation_names_are_closed_and_dot_cased() {
        let encoded = serde_json::to_string(&request()).unwrap();
        assert!(encoded.contains("\"operation\":\"draft.create\""));
        let invalid = encoded.replace("draft.create", "draft.exec");
        let unknown: AuthoringRequest = serde_json::from_str(&invalid).unwrap();
        assert_eq!(unknown.operation, AuthoringOperation::Unknown);
        assert_eq!(
            unknown.validate().unwrap_err().code,
            AuthoringErrorCode::UnknownOperation
        );
    }

    #[test]
    fn validates_protocol_ids_sequence_deadline_and_object_payload() {
        let mut value = request();
        assert!(value.validate().is_ok());
        value.protocol = "other/v1".into();
        assert_eq!(
            value.validate().unwrap_err().code,
            AuthoringErrorCode::BadProtocol
        );
        value = request();
        value.sequence = 0;
        assert_eq!(
            value.validate().unwrap_err().code,
            AuthoringErrorCode::BadRequest
        );
        value = request();
        value.deadline_ms = 99;
        assert_eq!(
            value.validate().unwrap_err().code,
            AuthoringErrorCode::BadRequest
        );
        value = request();
        value.payload = json!([]);
        assert_eq!(
            value.validate().unwrap_err().code,
            AuthoringErrorCode::BadRequest
        );
    }

    #[test]
    fn matches_frontend_envelope_id_bytes_and_closed_error_codes() {
        let mut oversized_utf8_id = request();
        oversized_utf8_id.request_id = "€".repeat(22);
        assert_eq!(
            oversized_utf8_id.validate().unwrap_err().code,
            AuthoringErrorCode::BadRequest
        );

        let mut envelope: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&request()).unwrap()).unwrap();
        envelope["unexpected"] = json!(true);
        assert!(serde_json::from_value::<AuthoringRequest>(envelope).is_err());

        assert_eq!(
            serde_json::to_value(AuthoringErrorCode::AuthoringDisabled).unwrap(),
            json!("AUTHORING_DISABLED")
        );
        assert_eq!(
            serde_json::to_value(AuthoringErrorCode::AppDisconnected).unwrap(),
            json!("APP_DISCONNECTED")
        );
    }

    #[test]
    fn rejects_oversized_serialized_request_at_the_transport_limit() {
        let mut value = request();
        value.payload = json!({"large": "x".repeat(MAX_AUTHORING_REQUEST_BYTES)});
        assert_eq!(
            value.validate().unwrap_err().code,
            AuthoringErrorCode::PayloadTooLarge
        );
    }

    #[test]
    fn response_is_correlated_to_exact_request_identity_and_sequence() {
        let req = request();
        let response = req.success_response(json!({"active": true}));
        assert!(response.validate_for(&req));
        let mut replay = response;
        replay.sequence += 1;
        assert!(!replay.validate_for(&req));
    }
}
