use tune_bytecode::{Opcode, function::Instruction};
use tune_runtime::value::{StructFields, Value};

use crate::execute_support::{read_reg, read_reg_ref, runtime_variant, write_reg};
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
            .unwrap_or(u32::MAX);
        let mut fields = if max_field == u32::MAX {
            Vec::new()
        } else {
            vec![Value::Unit; max_field as usize + 1]
        };
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
        let value = read_reg_ref(registers, instruction.b)
            .map_err(|error| self.fault_at(function_index, ip, error))?;
        let result = matches!(value, Value::Struct { owner, .. } if *owner == instruction.c);
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
        let site = self.field_site(function_index, ip, instruction.c)?;
        let value = read_reg_ref(registers, instruction.b)
            .map_err(|error| self.fault_at(function_index, ip, error))?;
        match value {
            Value::Struct { owner, fields } if *owner == site.owner => {
                let value = fields.get(site.field as usize).ok_or_else(|| {
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
        let site = self.field_site(function_index, ip, instruction.b)?;
        match self.at(function_index, ip, read_reg(registers, instruction.a))? {
            Value::Struct { owner, fields } if owner == site.owner => {
                let value = self.at(function_index, ip, read_reg(registers, instruction.c))?;
                if value.contains_state(fields.state()) {
                    return Err(self.fault_at(function_index, ip, VmError::RecursiveStructState));
                }
                fields
                    .set(site.field as usize, value)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::RegisterOutOfBounds))
            }
            _ => Err(self.fault_at(
                function_index,
                ip,
                VmError::UnsupportedOpcode(Opcode::FieldSet),
            )),
        }
    }

    fn field_site(
        &self,
        function_index: usize,
        ip: usize,
        site: u32,
    ) -> Result<&tune_bytecode::function::BytecodeFieldSite, VmFault> {
        self.artifact
            .functions
            .get(function_index)
            .and_then(|function| function.field_sites.get(site as usize))
            .ok_or_else(|| self.fault_at(function_index, ip, VmError::RegisterOutOfBounds))
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
        let value = read_reg_ref(registers, instruction.b)
            .map_err(|error| self.fault_at(function_index, ip, error))?;
        match value {
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

    pub(crate) fn execute_tuple_field(
        &self,
        function_index: usize,
        ip: usize,
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        let value = read_reg_ref(registers, instruction.b)
            .map_err(|error| self.fault_at(function_index, ip, error))?;
        match value {
            Value::Tuple(fields) => {
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
                VmError::UnsupportedOpcode(Opcode::TupleField),
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
        let value = read_reg_ref(registers, instruction.a)
            .map_err(|error| self.fault_at(function_index, ip, error))?;
        let Value::Variant { variant, .. } = value else {
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
            .find(|arm| runtime_variant(arm.variant) == *variant)
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
