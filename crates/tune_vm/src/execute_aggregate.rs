use tune_bytecode::{Opcode, function::Instruction};
use tune_runtime::value::{StructFields, Value};

use crate::execute_support::{read_reg, runtime_variant, write_reg};
use crate::{Vm, VmError, VmFault};

impl Vm {
    pub(crate) fn execute_tuple_build(
        &self,
        function_index: usize,
        ip: usize,
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        let function = self
            .artifact
            .functions
            .get(function_index)
            .ok_or_else(|| self.fault_at(function_index, ip, VmError::FunctionOutOfBounds))?;
        let site = function
            .tuple_sites
            .get(instruction.b as usize)
            .ok_or_else(|| self.fault_at(function_index, ip, VmError::CallSiteOutOfBounds))?;
        let values = site
            .items
            .iter()
            .map(|item| self.at(function_index, ip, read_reg(registers, *item)))
            .collect::<Result<Vec<_>, _>>()?;
        self.at(
            function_index,
            ip,
            write_reg(registers, instruction.a, Value::Tuple(values)),
        )
    }

    pub(crate) fn execute_struct_construct(
        &self,
        function_index: usize,
        ip: usize,
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        let function = self
            .artifact
            .functions
            .get(function_index)
            .ok_or_else(|| self.fault_at(function_index, ip, VmError::FunctionOutOfBounds))?;
        let site = function
            .struct_sites
            .get(instruction.b as usize)
            .ok_or_else(|| self.fault_at(function_index, ip, VmError::StructSiteOutOfBounds))?;
        let max_field = site
            .fields
            .iter()
            .map(|field| field.field)
            .max()
            .unwrap_or(0);
        let mut fields = vec![Value::Unit; max_field as usize + 1];
        for field in &site.fields {
            fields[field.field as usize] =
                self.at(function_index, ip, read_reg(registers, field.value))?;
        }
        let state = self.at(function_index, ip, self.alloc_state(site.state))?;
        self.at(
            function_index,
            ip,
            write_reg(
                registers,
                instruction.a,
                Value::Struct {
                    owner: site.owner,
                    fields: StructFields::new(state, fields),
                },
            ),
        )
    }

    pub(crate) fn execute_struct_is(
        &self,
        function_index: usize,
        ip: usize,
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        let value = self.at(function_index, ip, read_reg(registers, instruction.b))?;
        let result = matches!(value, Value::Struct { owner, .. } if owner == instruction.c);
        self.at(
            function_index,
            ip,
            write_reg(registers, instruction.a, Value::Bool(result)),
        )
    }

    pub(crate) fn execute_field_get(
        &self,
        function_index: usize,
        ip: usize,
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        match self.at(function_index, ip, read_reg(registers, instruction.b))? {
            Value::Struct { fields, .. } => {
                let value = fields.get(instruction.c as usize).ok_or_else(|| {
                    self.fault_at(function_index, ip, VmError::RegisterOutOfBounds)
                })?;
                self.at(
                    function_index,
                    ip,
                    write_reg(registers, instruction.a, value),
                )
            }
            _ => Err(self.fault_at(
                function_index,
                ip,
                VmError::UnsupportedOpcode(Opcode::FieldGet),
            )),
        }
    }

    pub(crate) fn execute_field_set(
        &self,
        function_index: usize,
        ip: usize,
        registers: &[Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        match self.at(function_index, ip, read_reg(registers, instruction.a))? {
            Value::Struct { fields, .. } => {
                let value = self.at(function_index, ip, read_reg(registers, instruction.c))?;
                fields
                    .set(instruction.b as usize, value)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::RegisterOutOfBounds))
            }
            _ => Err(self.fault_at(
                function_index,
                ip,
                VmError::UnsupportedOpcode(Opcode::FieldSet),
            )),
        }
    }

    pub(crate) fn execute_variant_construct(
        &self,
        function_index: usize,
        ip: usize,
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        let function = self
            .artifact
            .functions
            .get(function_index)
            .ok_or_else(|| self.fault_at(function_index, ip, VmError::FunctionOutOfBounds))?;
        let variant_site = function
            .variant_sites
            .get(instruction.b as usize)
            .ok_or_else(|| self.fault_at(function_index, ip, VmError::CallSiteOutOfBounds))?;
        let fields = variant_site
            .args
            .iter()
            .map(|arg| self.at(function_index, ip, read_reg(registers, *arg)))
            .collect::<Result<Vec<_>, _>>()?;
        self.at(
            function_index,
            ip,
            write_reg(
                registers,
                instruction.a,
                Value::Variant {
                    variant: runtime_variant(variant_site.variant),
                    fields,
                    propagation_frames: Vec::new(),
                },
            ),
        )
    }

    pub(crate) fn execute_variant_field(
        &self,
        function_index: usize,
        ip: usize,
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        match self.at(function_index, ip, read_reg(registers, instruction.b))? {
            Value::Variant { fields, .. } => {
                let value = fields.get(instruction.c as usize).cloned().ok_or_else(|| {
                    self.fault_at(function_index, ip, VmError::RegisterOutOfBounds)
                })?;
                self.at(
                    function_index,
                    ip,
                    write_reg(registers, instruction.a, value),
                )
            }
            _ => Err(self.fault_at(
                function_index,
                ip,
                VmError::UnsupportedOpcode(Opcode::VariantField),
            )),
        }
    }

    pub(crate) fn execute_match_variant(
        &self,
        function_index: usize,
        ip: usize,
        registers: &[Value],
        instruction: &Instruction,
    ) -> Result<usize, VmFault> {
        let Value::Variant { variant, .. } =
            self.at(function_index, ip, read_reg(registers, instruction.a))?
        else {
            return Err(self.fault_at(
                function_index,
                ip,
                VmError::UnsupportedOpcode(Opcode::MatchVariant),
            ));
        };
        let function = self
            .artifact
            .functions
            .get(function_index)
            .ok_or_else(|| self.fault_at(function_index, ip, VmError::FunctionOutOfBounds))?;
        let match_site = function
            .match_sites
            .get(instruction.b as usize)
            .ok_or_else(|| self.fault_at(function_index, ip, VmError::CallSiteOutOfBounds))?;
        if let Some(arm) = match_site
            .arms
            .iter()
            .find(|arm| runtime_variant(arm.variant) == variant)
        {
            return Ok(arm.target as usize);
        }
        if instruction.c == u32::MAX {
            return Err(self.fault_at(
                function_index,
                ip,
                VmError::UnsupportedOpcode(Opcode::MatchVariant),
            ));
        }
        Ok(instruction.c as usize)
    }
}
