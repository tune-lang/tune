use tune_ir::IrFunction;

use super::BytecodeLowerError;

pub(super) struct FlatFunction<'ir> {
    pub(super) function: &'ir IrFunction,
    pub(super) task_indices: Vec<u32>,
}

pub(super) fn flatten_functions(
    functions: &[IrFunction],
) -> Result<Vec<FlatFunction<'_>>, BytecodeLowerError> {
    let mut flat = Vec::new();
    for function in functions {
        flatten_function(function, &mut flat)?;
    }
    Ok(flat)
}

fn flatten_function<'ir>(
    function: &'ir IrFunction,
    flat: &mut Vec<FlatFunction<'ir>>,
) -> Result<u32, BytecodeLowerError> {
    let index = u32::try_from(flat.len()).map_err(|_| BytecodeLowerError::ConstantLimit)?;
    flat.push(FlatFunction {
        function,
        task_indices: Vec::new(),
    });
    let task_indices = function
        .task_functions
        .iter()
        .map(|task| flatten_function(task, flat))
        .collect::<Result<Vec<_>, _>>()?;
    flat[index as usize].task_indices = task_indices;
    Ok(index)
}
