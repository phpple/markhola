use std::path::PathBuf;

use serde::Serialize;

use crate::document::{DocumentSnapshot, DocumentTabSnapshot};

#[derive(Clone, Copy, Debug)]
pub(crate) enum EditCommand {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

#[derive(Clone, Debug)]
pub(crate) enum UserEvent {
    OpenFile(ActionContext),
    OpenPath(OpenPathRequest),
    ActivateDocument(u64),
    ActivateNextDocument,
    ActivatePreviousDocument,
    CloseDocument(u64),
    CloseCurrentDocument,
    CloseOtherDocuments,
    CloseAllDocuments,
    ShellReady,
    RecoverShell(String),
    OpenExternal(String),
    SaveDocument,
    SaveDocumentAs,
    ExportPdf,
    ExportHtml,
    PrintDocument,
    OpenFind,
    EditCommand(EditCommand),
    ToggleMode,
    EditorChanged(String),
    ShowAbout,
    OpenDocumentation,
    Exit,
}

#[derive(Clone, Debug)]
pub(crate) enum PendingChangesAction {
    Save,
    Discard,
    Cancel,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StatusPayload<'a> {
    pub(crate) message: &'a str,
    pub(crate) level: &'a str,
    pub(crate) action_path: Option<&'a str>,
    pub(crate) action_label: Option<&'a str>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct WorkspacePresentation {
    pub(crate) tabs: Vec<DocumentTabSnapshot>,
    pub(crate) active_document: Option<DocumentSnapshot>,
    pub(crate) status_message: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ActionContext {
    pub(crate) event_id: u64,
    pub(crate) source: &'static str,
}

#[derive(Clone, Debug)]
pub(crate) struct OpenPathRequest {
    pub(crate) ctx: ActionContext,
    pub(crate) path: PathBuf,
}
