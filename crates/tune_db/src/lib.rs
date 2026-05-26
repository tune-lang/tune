pub mod ids;
pub mod interner;
pub mod source;

pub use ids::*;

#[derive(Default)]
pub struct TuneDb {
    pub files: Vec<source::SourceFile>,
}

impl TuneDb {
    pub fn add_file(&mut self, path: impl Into<String>, text: impl Into<String>) -> FileId {
        let id = FileId(self.files.len() as u32);
        self.files.push(source::SourceFile {
            id,
            path: path.into(),
            text: text.into(),
        });
        id
    }
}
