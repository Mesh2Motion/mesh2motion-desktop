use crate::{AuthoringHandler, PoseImageChunkHandler, PoseObservationHandler, SnapshotStore};

#[derive(Debug)]
pub struct HostError;

pub struct McpHost;

impl McpHost {
    pub fn start(_store: SnapshotStore) -> Result<Self, HostError> {
        Err(HostError)
    }
    pub fn start_with_authoring(
        _store: SnapshotStore,
        _authoring_handler: Option<AuthoringHandler>,
    ) -> Result<Self, HostError> {
        Err(HostError)
    }
    pub fn start_with_handlers(
        _store: SnapshotStore,
        _authoring_handler: Option<AuthoringHandler>,
        _observation_handler: Option<PoseObservationHandler>,
        _image_chunk_handler: Option<PoseImageChunkHandler>,
    ) -> Result<Self, HostError> {
        Err(HostError)
    }
    pub fn endpoint(&self) -> &std::path::Path {
        std::path::Path::new("")
    }

    pub fn socket_available(&self) -> bool {
        false
    }
}
