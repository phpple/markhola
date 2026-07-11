use std::path::Path;

use crate::document::{ActiveDocument, DocumentSnapshot, DocumentTabSnapshot};

#[derive(Debug, Default)]
pub struct DocumentWorkspace {
    documents: Vec<ActiveDocument>,
    active_index: Option<usize>,
    next_document_id: u64,
}

impl DocumentWorkspace {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
            active_index: None,
            next_document_id: 1,
        }
    }

    pub fn next_document_id(&mut self) -> u64 {
        let id = self.next_document_id;
        self.next_document_id += 1;
        id
    }

    pub fn active_document(&self) -> Option<&ActiveDocument> {
        self.active_index.and_then(|index| self.documents.get(index))
    }

    pub fn active_document_mut(&mut self) -> Option<&mut ActiveDocument> {
        self.active_index.and_then(|index| self.documents.get_mut(index))
    }

    pub fn document_by_id_mut(&mut self, document_id: u64) -> Option<&mut ActiveDocument> {
        self.documents.iter_mut().find(|document| document.id() == document_id)
    }

    pub fn active_document_id(&self) -> Option<u64> {
        self.active_document().map(ActiveDocument::id)
    }

    pub fn document_ids(&self) -> Vec<u64> {
        self.documents.iter().map(ActiveDocument::id).collect()
    }

    pub fn other_document_ids(&self, active_document_id: u64) -> Vec<u64> {
        self.documents
            .iter()
            .filter(|document| document.id() != active_document_id)
            .map(ActiveDocument::id)
            .collect()
    }

    pub fn tab_snapshots(&self) -> Vec<DocumentTabSnapshot> {
        self.documents
            .iter()
            .enumerate()
            .map(|(index, document)| document.tab_snapshot(Some(index) == self.active_index))
            .collect()
    }

    pub fn active_document_snapshot(&self) -> Option<DocumentSnapshot> {
        self.active_document().map(ActiveDocument::snapshot)
    }

    pub fn active_window_title(&self) -> Option<String> {
        self.active_document().map(ActiveDocument::window_title)
    }

    pub fn open_document(&mut self, document: ActiveDocument) -> WorkspaceOpenResult {
        if let Some(index) = self
            .documents
            .iter()
            .position(|existing| existing.canonical_path() == document.canonical_path())
        {
            self.active_index = Some(index);
            return WorkspaceOpenResult::ActivatedExisting(self.documents[index].id());
        }

        let document_id = document.id();
        self.documents.push(document);
        self.active_index = Some(self.documents.len() - 1);
        WorkspaceOpenResult::OpenedNew(document_id)
    }

    pub fn activate_document(&mut self, document_id: u64) -> bool {
        let Some(index) = self.documents.iter().position(|document| document.id() == document_id) else {
            return false;
        };
        self.active_index = Some(index);
        true
    }

    pub fn activate_next_document(&mut self) -> bool {
        let Some(active_index) = self.active_index else {
            return false;
        };
        if self.documents.len() < 2 {
            return false;
        }

        self.active_index = Some((active_index + 1) % self.documents.len());
        true
    }

    pub fn activate_previous_document(&mut self) -> bool {
        let Some(active_index) = self.active_index else {
            return false;
        };
        if self.documents.len() < 2 {
            return false;
        }

        self.active_index = Some((active_index + self.documents.len() - 1) % self.documents.len());
        true
    }

    pub fn close_document(&mut self, document_id: u64) -> Option<ActiveDocument> {
        let index = self.documents.iter().position(|document| document.id() == document_id)?;
        let removed = self.documents.remove(index);

        self.active_index = if self.documents.is_empty() {
            None
        } else if Some(index) == self.active_index {
            Some(index.min(self.documents.len() - 1))
        } else {
            self.active_index.map(|active_index| {
                if active_index > index {
                    active_index - 1
                } else {
                    active_index
                }
            })
        };

        Some(removed)
    }

    pub fn find_by_path(&self, path: &Path) -> Option<u64> {
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        self.documents
            .iter()
            .find(|document| document.canonical_path() == canonical.as_path())
            .map(ActiveDocument::id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceOpenResult {
    ActivatedExisting(u64),
    OpenedNew(u64),
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::document::ActiveDocument;

    use super::{DocumentWorkspace, WorkspaceOpenResult};

    fn document(id: u64, path: &str) -> ActiveDocument {
        ActiveDocument::open_with_id(
            id,
            PathBuf::from(path),
            format!("# {path}\ncontent"),
            "file:///tmp/".to_string(),
        )
    }

    #[test]
    fn opens_multiple_documents_and_tracks_active_one() {
        let mut workspace = DocumentWorkspace::new();

        assert_eq!(workspace.open_document(document(1, "/tmp/one.md")), WorkspaceOpenResult::OpenedNew(1));
        assert_eq!(workspace.open_document(document(2, "/tmp/two.md")), WorkspaceOpenResult::OpenedNew(2));

        assert_eq!(workspace.active_document_id(), Some(2));
        assert_eq!(workspace.tab_snapshots().len(), 2);
    }

    #[test]
    fn reopening_same_path_activates_existing_document() {
        let mut workspace = DocumentWorkspace::new();

        workspace.open_document(document(1, "/tmp/one.md"));
        let result = workspace.open_document(document(2, "/tmp/one.md"));

        assert_eq!(result, WorkspaceOpenResult::ActivatedExisting(1));
        assert_eq!(workspace.tab_snapshots().len(), 1);
        assert_eq!(workspace.active_document_id(), Some(1));
    }

    #[test]
    fn closing_active_document_selects_neighbor() {
        let mut workspace = DocumentWorkspace::new();

        workspace.open_document(document(1, "/tmp/one.md"));
        workspace.open_document(document(2, "/tmp/two.md"));
        workspace.open_document(document(3, "/tmp/three.md"));

        workspace.activate_document(2);
        let closed = workspace.close_document(2).unwrap();

        assert_eq!(closed.id(), 2);
        assert_eq!(workspace.active_document_id(), Some(3));
        assert_eq!(workspace.tab_snapshots().len(), 2);
    }
}
