use crate::ids::FileId;
use tune_diagnostics::render::{SourceProvider, SourceView};

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub id: FileId,
    pub path: String,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, path: impl Into<String>, text: impl Into<String>) -> Option<FileId> {
        let index = u32::try_from(self.files.len()).ok()?;
        let id = FileId(index);
        self.files.push(SourceFile {
            id,
            path: path.into(),
            text: text.into(),
        });
        Some(id)
    }

    #[must_use]
    pub fn get(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize).filter(|file| file.id == id)
    }

    pub fn set_text(&mut self, id: FileId, text: impl Into<String>) -> bool {
        let Some(file) = self
            .files
            .get_mut(id.0 as usize)
            .filter(|file| file.id == id)
        else {
            return false;
        };
        file.text = text.into();
        true
    }

    #[must_use]
    pub fn find_by_path(&self, path: &str) -> Option<FileId> {
        self.files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &SourceFile> {
        self.files.iter()
    }
}

impl SourceProvider for SourceMap {
    fn source(&self, file: FileId) -> Option<SourceView<'_>> {
        let source = self.get(file)?;
        Some(SourceView {
            path: &source.path,
            text: &source.text,
        })
    }
}
