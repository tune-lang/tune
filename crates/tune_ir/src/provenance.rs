use tune_diagnostics::Span;

use crate::IrOp;

impl IrOp {
    #[must_use]
    pub const fn provenance_span(&self) -> Option<Span> {
        match self {
            Self::AddInt { span, .. }
            | Self::SubInt { span, .. }
            | Self::MulInt { span, .. }
            | Self::DivInt { span, .. }
            | Self::RemInt { span, .. }
            | Self::BitAndInt { span, .. }
            | Self::BitOrInt { span, .. }
            | Self::BitXorInt { span, .. }
            | Self::ShiftLeftInt { span, .. }
            | Self::ShiftRightInt { span, .. }
            | Self::AddSizeChecked { span, .. }
            | Self::SubSizeChecked { span, .. }
            | Self::MulSizeChecked { span, .. }
            | Self::DivSize { span, .. }
            | Self::RemSize { span, .. }
            | Self::BitAndSize { span, .. }
            | Self::BitOrSize { span, .. }
            | Self::BitXorSize { span, .. }
            | Self::ShiftLeftSize { span, .. }
            | Self::ShiftRightSize { span, .. }
            | Self::RangeInt { span, .. }
            | Self::NegInt { span, .. }
            | Self::NotBool { span, .. }
            | Self::BitNotInt { span, .. }
            | Self::BitNotSize { span, .. }
            | Self::NoneCheck { span, .. }
            | Self::GreaterInt { span, .. }
            | Self::CompareInt { span, .. }
            | Self::SubFloat { span, .. }
            | Self::MulFloat { span, .. }
            | Self::DivFloat { span, .. }
            | Self::GreaterFloat { span, .. }
            | Self::CompareFloat { span, .. }
            | Self::GreaterSize { span, .. }
            | Self::CompareSize { span, .. }
            | Self::ByteBinary { span, .. }
            | Self::GetField { span, .. }
            | Self::SetField { span, .. }
            | Self::VariantConstruct { span, .. }
            | Self::StructConstruct { span, .. }
            | Self::StructIs { span, .. }
            | Self::CallDirect { span, .. }
            | Self::CallMember { span, .. }
            | Self::CallableValue { span, .. }
            | Self::CallBound { span, .. }
            | Self::StringLen { span, .. }
            | Self::StringGet { span, .. }
            | Self::Branch { span, .. }
            | Self::MatchVariant { span, .. }
            | Self::ResultPropagate { span, .. } => *span,
            Self::Spawn { span, .. } | Self::TaskJoin { span, .. } => *span,
            Self::Panic { span, .. } => *span,
            _ => None,
        }
    }
}
