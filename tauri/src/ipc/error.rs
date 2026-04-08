/// Typed error returned by all Tauri IPC commands.
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl From<String> for IpcError {
    fn from(s: String) -> Self {
        IpcError::Other(s)
    }
}

// Tauri requires command errors to implement Serialize
impl serde::Serialize for IpcError {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.to_string())
    }
}
