use tune_db::TuneDb;
use tune_diagnostics::{ByteOffset, Diagnostic, Severity, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDiagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
}

#[must_use]
pub fn diagnostic(db: &TuneDb, diagnostic: &Diagnostic) -> Option<LspDiagnostic> {
    Some(LspDiagnostic {
        range: range(db, diagnostic.primary_span())?,
        severity: severity(diagnostic.severity),
        code: diagnostic.code.to_string(),
        message: diagnostic.title.clone(),
    })
}

#[must_use]
pub fn diagnostic_hover(diagnostic: &Diagnostic) -> String {
    tune_diagnostics::render::render(
        diagnostic,
        tune_diagnostics::render::DiagnosticRenderMode::LspHoverCompact,
    )
}

#[must_use]
pub fn range(db: &TuneDb, span: Span) -> Option<Range> {
    let source = db.source(span.file)?;
    Some(Range {
        start: position(&source.text, span.start.get())?,
        end: position(&source.text, span.end.get())?,
    })
}

#[must_use]
pub fn byte_offset(db: &TuneDb, file: tune_db::FileId, position: Position) -> Option<ByteOffset> {
    let source = db.source(file)?;
    offset(&source.text, position).and_then(|offset| offset.try_into().ok().map(ByteOffset::new))
}

fn severity(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::Error,
        Severity::Warning => DiagnosticSeverity::Warning,
        Severity::Info => DiagnosticSeverity::Information,
    }
}

pub fn position(text: &str, byte_offset: u32) -> Option<Position> {
    let offset = usize::try_from(byte_offset).ok()?;
    if offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }

    let mut line = 0_u32;
    let mut line_start = 0_usize;
    for (index, byte) in text.bytes().enumerate() {
        if index >= offset {
            break;
        }
        if byte == b'\n' {
            line = line.checked_add(1)?;
            line_start = index + 1;
        }
    }

    let character = text[line_start..offset]
        .encode_utf16()
        .count()
        .try_into()
        .ok()?;
    Some(Position { line, character })
}

fn offset(text: &str, position: Position) -> Option<usize> {
    let target_line = usize::try_from(position.line).ok()?;
    let target_character = usize::try_from(position.character).ok()?;
    let mut line = 0_usize;
    let mut line_start = 0_usize;
    for (index, byte) in text.bytes().enumerate() {
        if line == target_line {
            break;
        }
        if byte == b'\n' {
            line += 1;
            line_start = index + 1;
        }
    }
    if line != target_line {
        return None;
    }

    let line_end = text[line_start..]
        .find('\n')
        .map_or(text.len(), |index| line_start + index);
    let line_text = &text[line_start..line_end];
    let mut utf16_units = 0_usize;
    for (relative_index, character) in line_text.char_indices() {
        if utf16_units == target_character {
            return Some(line_start + relative_index);
        }
        utf16_units += character.len_utf16();
        if utf16_units > target_character {
            return None;
        }
    }
    if utf16_units == target_character {
        Some(line_end)
    } else {
        None
    }
}
