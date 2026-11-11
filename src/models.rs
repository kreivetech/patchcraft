use serde::{Deserialize, Serialize};

/// Represents an individual patch operation on a file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PatchObj {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub anchor: String,
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub rename_to: String,
}

/// A container for multiple file patch operations belonging to a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FileObject {
    #[serde(default)]
    pub session_id: String,
    pub summary: Option<String>,
    pub patch: Vec<PatchObj>,
}

/// The response payload wrapper for JSON format compatibility.
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponsePayload {
    pub file: FileObject,
}

/// The specific result status of applying a patch action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchActionKind {
    Created(String),
    Edited(String),
    Renamed { from: String, to: String },
    Deleted(String),
    Appended { path: String, anchor: String },
    Failed { path: String, reason: String },
    SkippedUnsafe(String),
}

/// The result returned after executing a patch against a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchActionResult {
    pub path: String,
    pub kind: PatchActionKind,
}
