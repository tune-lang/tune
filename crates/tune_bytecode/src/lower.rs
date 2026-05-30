use std::collections::HashMap;

mod calls;
mod compare;
mod context;
mod control;
mod error;
mod flatten;
mod frame;
mod numeric;
mod op;
mod sequence;
mod string;

use crate::artifact::{BytecodeArtifact, BytecodeConst};
use crate::function::{BytecodeFunction, BytecodeStructLayout};
use crate::lower_tables::{
    block_offsets, callable_function_indices, function_indices, member_function_indices,
};
use crate::provenance::BytecodeFunctionProvenance;
use tune_hir::{HirId, MemberId};
use tune_ir::IrFunction;

pub(crate) use self::context::FunctionLowerer;
pub use self::error::BytecodeLowerError;
use self::flatten::flatten_functions;

pub fn lower_ir_functions(
    functions: &[IrFunction],
) -> Result<BytecodeArtifact, BytecodeLowerError> {
    let flat = flatten_functions(functions)?;
    let function_indices = function_indices(flat.iter().map(|function| function.function))?;
    let member_indices = member_function_indices(flat.iter().map(|function| function.function))?;
    let callable_indices =
        callable_function_indices(flat.iter().map(|function| function.function))?;
    let mut constants = Vec::new();
    let functions = flat
        .iter()
        .map(|function| {
            lower_ir_function_with_constants(
                function.function,
                &function_indices,
                &member_indices,
                &callable_indices,
                &function.task_indices,
                &mut constants,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BytecodeArtifact {
        entry_function: (!functions.is_empty()).then_some(0),
        struct_layouts: lower_struct_layouts(flat.iter().map(|function| function.function)),
        functions,
        constants,
    })
}

fn lower_struct_layouts<'ir>(
    functions: impl IntoIterator<Item = &'ir IrFunction>,
) -> Vec<BytecodeStructLayout> {
    let mut layouts = Vec::<BytecodeStructLayout>::new();
    for layout in functions
        .into_iter()
        .flat_map(|function| &function.struct_layouts)
    {
        if layouts
            .iter()
            .any(|candidate| candidate.owner == layout.owner.0)
        {
            continue;
        }
        layouts.push(BytecodeStructLayout {
            owner: layout.owner.0,
            fields: layout.fields.iter().map(|field| field.0).collect(),
        });
    }
    layouts
}

pub fn lower_ir_function(function: &IrFunction) -> Result<BytecodeFunction, BytecodeLowerError> {
    let mut constants = Vec::new();
    let function_indices = function_indices(std::iter::once(function))?;
    let member_indices = member_function_indices(std::iter::once(function))?;
    let callable_indices = callable_function_indices(std::iter::once(function))?;
    lower_ir_function_with_constants(
        function,
        &function_indices,
        &member_indices,
        &callable_indices,
        &[],
        &mut constants,
    )
}

fn lower_ir_function_with_constants(
    function: &IrFunction,
    function_indices: &HashMap<HirId, u32>,
    member_indices: &HashMap<MemberId, u32>,
    callable_indices: &HashMap<tune_hir::ExprId, u32>,
    task_indices: &[u32],
    constants: &mut Vec<BytecodeConst>,
) -> Result<BytecodeFunction, BytecodeLowerError> {
    let block_offsets = block_offsets(function)?;
    let mut lowerer = FunctionLowerer {
        function,
        function_indices,
        member_indices,
        callable_indices,
        task_indices,
        block_offsets,
        constants,
        call_sites: Vec::new(),
        bound_call_sites: Vec::new(),
        host_call_sites: Vec::new(),
        callable_sites: Vec::new(),
        task_sites: Vec::new(),
        struct_sites: Vec::new(),
        field_sites: Vec::new(),
        variant_sites: Vec::new(),
        match_sites: Vec::new(),
        for_sites: Vec::new(),
        panic_sites: Vec::new(),
        tuple_sites: Vec::new(),
        string_sites: Vec::new(),
        instructions: Vec::new(),
        instruction_spans: Vec::new(),
    };
    for block in &function.blocks {
        for op in &block.ops {
            lowerer.lower_op(op)?;
            lowerer
                .instruction_spans
                .resize(lowerer.instructions.len(), op.provenance_span());
        }
    }
    Ok(BytecodeFunction {
        name: function.name.clone(),
        provenance: BytecodeFunctionProvenance {
            span: function.span,
            instruction_spans: lowerer.instruction_spans,
        },
        generic_param_count: u32::try_from(function.type_params.len())
            .map_err(|_| BytecodeLowerError::ConstantLimit)?,
        param_count: function.params,
        register_count: function.regs,
        local_count: function.locals,
        frame: frame::infer_frame_layout(function),
        call_sites: lowerer.call_sites,
        bound_call_sites: lowerer.bound_call_sites,
        host_call_sites: lowerer.host_call_sites,
        callable_sites: lowerer.callable_sites,
        task_sites: lowerer.task_sites,
        struct_sites: lowerer.struct_sites,
        field_sites: lowerer.field_sites,
        variant_sites: lowerer.variant_sites,
        match_sites: lowerer.match_sites,
        for_sites: lowerer.for_sites,
        panic_sites: lowerer.panic_sites,
        tuple_sites: lowerer.tuple_sites,
        string_sites: lowerer.string_sites,
        instructions: lowerer.instructions,
    })
}
