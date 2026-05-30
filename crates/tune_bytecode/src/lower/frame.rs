use tune_ir::{IrByteBinary, IrFunction, IrOp, Reg};
use tune_shape::Shape;

use crate::function::BytecodeFrameLayout;

pub(super) fn infer_frame_layout(function: &IrFunction) -> BytecodeFrameLayout {
    let mut layout = BytecodeFrameLayout::unknown(function.params, function.regs, function.locals);

    for op in function.blocks.iter().flat_map(|block| &block.ops) {
        match op {
            IrOp::LoadConst { dst, shape, .. } => set_register(&mut layout, *dst, shape.clone()),
            IrOp::LoadLocal { dst, local, .. } => {
                let shape = layout
                    .locals
                    .get(local.0 as usize)
                    .cloned()
                    .unwrap_or(Shape::Hole);
                set_register(&mut layout, *dst, shape);
            }
            IrOp::StoreLocal { local, value, .. } => {
                let shape = register_shape(&layout, *value);
                if let Some(slot) = layout.locals.get_mut(local.0 as usize) {
                    *slot = slot.clone().join(shape);
                }
            }
            IrOp::Move { dst, src, .. } => {
                let shape = register_shape(&layout, *src);
                set_register(&mut layout, *dst, shape);
            }
            IrOp::AddInt { dst, .. }
            | IrOp::SubInt { dst, .. }
            | IrOp::MulInt { dst, .. }
            | IrOp::DivInt { dst, .. }
            | IrOp::RemInt { dst, .. }
            | IrOp::BitAndInt { dst, .. }
            | IrOp::BitOrInt { dst, .. }
            | IrOp::BitXorInt { dst, .. }
            | IrOp::ShiftLeftInt { dst, .. }
            | IrOp::ShiftRightInt { dst, .. }
            | IrOp::NegInt { dst, .. }
            | IrOp::BitNotInt { dst, .. } => set_register(&mut layout, *dst, Shape::Int),
            IrOp::AddFloat { dst, .. }
            | IrOp::SubFloat { dst, .. }
            | IrOp::MulFloat { dst, .. }
            | IrOp::DivFloat { dst, .. } => set_register(&mut layout, *dst, Shape::Float),
            IrOp::AddSizeChecked { dst, .. }
            | IrOp::SubSizeChecked { dst, .. }
            | IrOp::MulSizeChecked { dst, .. }
            | IrOp::DivSize { dst, .. }
            | IrOp::RemSize { dst, .. }
            | IrOp::BitAndSize { dst, .. }
            | IrOp::BitOrSize { dst, .. }
            | IrOp::BitXorSize { dst, .. }
            | IrOp::ShiftLeftSize { dst, .. }
            | IrOp::ShiftRightSize { dst, .. }
            | IrOp::BitNotSize { dst, .. } => set_register(&mut layout, *dst, Shape::Size),
            IrOp::AddByteWrap { dst, .. } => {
                set_register(&mut layout, *dst, Shape::Byte);
            }
            IrOp::ByteBinary { dst, op, .. } => set_register(
                &mut layout,
                *dst,
                if byte_binary_returns_bool(*op) {
                    Shape::Bool
                } else {
                    Shape::Byte
                },
            ),
            IrOp::GreaterInt { dst, .. }
            | IrOp::CompareInt { dst, .. }
            | IrOp::GreaterFloat { dst, .. }
            | IrOp::CompareFloat { dst, .. }
            | IrOp::GreaterSize { dst, .. }
            | IrOp::CompareSize { dst, .. }
            | IrOp::StructIs { dst, .. }
            | IrOp::NotBool { dst, .. }
            | IrOp::NoneCheck { dst, .. } => set_register(&mut layout, *dst, Shape::Bool),
            IrOp::RangeInt { dst, .. } => {
                set_register(&mut layout, *dst, Shape::Range(Box::new(Shape::Int)));
            }
            IrOp::SeqBuild { dst, element_shape } => {
                set_register(
                    &mut layout,
                    *dst,
                    Shape::Sequence(Box::new(element_shape.clone())),
                );
            }
            IrOp::StringBuild { dst, .. }
            | IrOp::StringLen { dst, .. }
            | IrOp::StringGet { dst, .. } => set_string_op_shape(&mut layout, op, *dst),
            IrOp::Spawn { dst, .. } => {
                set_register(&mut layout, *dst, Shape::Task(Box::new(Shape::Hole)))
            }
            _ => {}
        }
    }

    layout
}

fn set_string_op_shape(layout: &mut BytecodeFrameLayout, op: &IrOp, dst: Reg) {
    if matches!(op, IrOp::StringLen { .. }) {
        set_register(layout, dst, Shape::Size);
    } else {
        set_register(layout, dst, Shape::String);
    }
}

fn set_register(layout: &mut BytecodeFrameLayout, reg: Reg, shape: Shape) {
    if let Some(slot) = layout.registers.get_mut(reg.0 as usize) {
        *slot = slot.clone().join(shape);
    }
}

fn register_shape(layout: &BytecodeFrameLayout, reg: Reg) -> Shape {
    layout
        .registers
        .get(reg.0 as usize)
        .cloned()
        .unwrap_or(Shape::Hole)
}

const fn byte_binary_returns_bool(op: IrByteBinary) -> bool {
    matches!(
        op,
        IrByteBinary::Greater
            | IrByteBinary::Equal
            | IrByteBinary::NotEqual
            | IrByteBinary::Less
            | IrByteBinary::LessEqual
            | IrByteBinary::GreaterEqual
    )
}
