use std::collections::BTreeMap;

use tune_db::{FileId, TuneDb};

#[derive(Debug, Default)]
pub struct DocumentSet {
    files_by_path: BTreeMap<String, FileId>,
}

impl DocumentSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(
        &mut self,
        db: &mut TuneDb,
        path: impl Into<String>,
        text: impl Into<String>,
    ) -> Option<FileId> {
        let path = path.into();
        let text = text.into();
        if let Some(file) = self
            .files_by_path
            .get(&path)
            .copied()
            .or_else(|| db.file_by_path(&path))
        {
            if !db.set_file_text(file, text) {
                return None;
            }
            self.files_by_path.insert(path, file);
            return Some(file);
        }

        let file = db.add_file(path.clone(), text)?;
        self.files_by_path.insert(path, file);
        Some(file)
    }

    pub fn change(
        &mut self,
        db: &mut TuneDb,
        path: impl AsRef<str>,
        text: impl Into<String>,
    ) -> Option<FileId> {
        let path = path.as_ref();
        let file = self
            .files_by_path
            .get(path)
            .copied()
            .or_else(|| db.file_by_path(path))?;
        if !db.set_file_text(file, text) {
            return None;
        }
        Some(file)
    }

    pub fn close(&mut self, path: impl AsRef<str>) -> Option<FileId> {
        self.files_by_path.remove(path.as_ref())
    }

    #[must_use]
    pub fn file(&self, path: impl AsRef<str>) -> Option<FileId> {
        self.files_by_path.get(path.as_ref()).copied()
    }
}
