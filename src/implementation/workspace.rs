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
        self.active_index
            .and_then(|index| self.documents.get(index))
    }

    pub fn active_document_mut(&mut self) -> Option<&mut ActiveDocument> {
        self.active_index
            .and_then(|index| self.documents.get_mut(index))
    }

    pub fn document_by_id_mut(&mut self, document_id: u64) -> Option<&mut ActiveDocument> {
        self.documents
            .iter_mut()
            .find(|document| document.id() == document_id)
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
        let Some(index) = self
            .documents
            .iter()
            .position(|document| document.id() == document_id)
        else {
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
        let index = self
            .documents
            .iter()
            .position(|document| document.id() == document_id)?;
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

    pub fn find_by_path_excluding(&self, path: &Path, excluded_document_id: u64) -> Option<u64> {
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        self.documents
            .iter()
            .find(|document| {
                document.id() != excluded_document_id
                    && document.canonical_path() == canonical.as_path()
            })
            .map(ActiveDocument::id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceOpenResult {
    ActivatedExisting(u64),
    OpenedNew(u64),
}
