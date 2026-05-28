use tune_diagnostics::Span;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BytecodeFunctionProvenance {
    pub span: Option<Span>,
    pub instruction_spans: Vec<Option<Span>>,
}

impl BytecodeFunctionProvenance {
    #[must_use]
    pub fn instruction_span(&self, instruction: u32) -> Option<Span> {
        self.instruction_spans
            .get(instruction as usize)
            .copied()
            .flatten()
            .or(self.span)
    }
}
