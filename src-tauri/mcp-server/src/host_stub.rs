use crate::SnapshotStore;

#[derive(Debug)]
pub struct HostError;

pub struct McpHost;

impl McpHost {
    pub fn start(_store: SnapshotStore) -> Result<Self, HostError> {
        Err(HostError)
    }
    pub fn endpoint(&self) -> &std::path::Path {
        std::path::Path::new("")
    }
}
