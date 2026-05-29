use tune_bytecode::{Opcode, function::Instruction};
use tune_runtime::value::Value;

use crate::execute_support::{read_reg_ref, write_reg};
use crate::{Vm, VmError, VmFault};

impl Vm {
    pub(crate) fn execute_int_comparison(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<(), VmFault> {
        let left = read_reg_ref(registers, op.b)
            .map_err(|error| self.fault_at(function, instruction, error))?;
        let right = read_reg_ref(registers, op.c)
            .map_err(|error| self.fault_at(function, instruction, error))?;
        let left = match left {
            Value::Int(left) => *left,
            _ => {
                return Err(self.fault_at(
                    function,
                    instruction,
                    VmError::UnsupportedOpcode(op.opcode),
                ));
            }
        };
        let right = match right {
            Value::Int(right) => *right,
            _ => {
                return Err(self.fault_at(
                    function,
                    instruction,
                    VmError::UnsupportedOpcode(op.opcode),
                ));
            }
        };
        let result = self.at(function, instruction, compare_int(op.opcode, left, right))?;
        self.at(
            function,
            instruction,
            write_reg(registers, op.a, Value::Bool(result)),
        )
    }

    pub(crate) fn execute_float_comparison(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<(), VmFault> {
        let left = read_reg_ref(registers, op.b)
            .map_err(|error| self.fault_at(function, instruction, error))?;
        let right = read_reg_ref(registers, op.c)
            .map_err(|error| self.fault_at(function, instruction, error))?;
        let left = match left {
            Value::Float(left) => *left,
            _ => {
                return Err(self.fault_at(
                    function,
                    instruction,
                    VmError::UnsupportedOpcode(op.opcode),
                ));
            }
        };
        let right = match right {
            Value::Float(right) => *right,
            _ => {
                return Err(self.fault_at(
                    function,
                    instruction,
                    VmError::UnsupportedOpcode(op.opcode),
                ));
            }
        };
        let result = self.at(function, instruction, compare_float(op.opcode, left, right))?;
        self.at(
            function,
            instruction,
            write_reg(registers, op.a, Value::Bool(result)),
        )
    }

    pub(crate) fn execute_size_comparison(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<(), VmFault> {
        let left = read_reg_ref(registers, op.b)
            .map_err(|error| self.fault_at(function, instruction, error))?;
        let right = read_reg_ref(registers, op.c)
            .map_err(|error| self.fault_at(function, instruction, error))?;
        let left = match left {
            Value::Size(left) => *left,
            _ => {
                return Err(self.fault_at(
                    function,
                    instruction,
                    VmError::UnsupportedOpcode(op.opcode),
                ));
            }
        };
        let right = match right {
            Value::Size(right) => *right,
            _ => {
                return Err(self.fault_at(
                    function,
                    instruction,
                    VmError::UnsupportedOpcode(op.opcode),
                ));
            }
        };
        let result = self.at(function, instruction, compare_size(op.opcode, left, right))?;
        self.at(
            function,
            instruction,
            write_reg(registers, op.a, Value::Bool(result)),
        )
    }

    pub(crate) fn execute_byte_comparison(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<(), VmFault> {
        let left = read_reg_ref(registers, op.b)
            .map_err(|error| self.fault_at(function, instruction, error))?;
        let right = read_reg_ref(registers, op.c)
            .map_err(|error| self.fault_at(function, instruction, error))?;
        let left = match left {
            Value::Byte(left) => *left,
            _ => {
                return Err(self.fault_at(
                    function,
                    instruction,
                    VmError::UnsupportedOpcode(op.opcode),
                ));
            }
        };
        let right = match right {
            Value::Byte(right) => *right,
            _ => {
                return Err(self.fault_at(
                    function,
                    instruction,
                    VmError::UnsupportedOpcode(op.opcode),
                ));
            }
        };
        let result = self.at(function, instruction, compare_byte(op.opcode, left, right))?;
        self.at(
            function,
            instruction,
            write_reg(registers, op.a, Value::Bool(result)),
        )
    }
}

fn compare_int(opcode: Opcode, left: i64, right: i64) -> Result<bool, VmError> {
    match opcode {
        Opcode::GreaterInt => Ok(left > right),
        Opcode::EqualInt => Ok(left == right),
        Opcode::NotEqualInt => Ok(left != right),
        Opcode::LessInt => Ok(left < right),
        Opcode::LessEqualInt => Ok(left <= right),
        Opcode::GreaterEqualInt => Ok(left >= right),
        _ => Err(VmError::UnsupportedOpcode(opcode)),
    }
}

fn compare_float(opcode: Opcode, left: f64, right: f64) -> Result<bool, VmError> {
    match opcode {
        Opcode::GreaterFloat => Ok(left > right),
        Opcode::EqualFloat => Ok(left == right),
        Opcode::NotEqualFloat => Ok(left != right),
        Opcode::LessFloat => Ok(left < right),
        Opcode::LessEqualFloat => Ok(left <= right),
        Opcode::GreaterEqualFloat => Ok(left >= right),
        _ => Err(VmError::UnsupportedOpcode(opcode)),
    }
}

fn compare_size(opcode: Opcode, left: u64, right: u64) -> Result<bool, VmError> {
    match opcode {
        Opcode::GreaterSize => Ok(left > right),
        Opcode::EqualSize => Ok(left == right),
        Opcode::NotEqualSize => Ok(left != right),
        Opcode::LessSize => Ok(left < right),
        Opcode::LessEqualSize => Ok(left <= right),
        Opcode::GreaterEqualSize => Ok(left >= right),
        _ => Err(VmError::UnsupportedOpcode(opcode)),
    }
}

fn compare_byte(opcode: Opcode, left: u8, right: u8) -> Result<bool, VmError> {
    match opcode {
        Opcode::GreaterByte => Ok(left > right),
        Opcode::EqualByte => Ok(left == right),
        Opcode::NotEqualByte => Ok(left != right),
        Opcode::LessByte => Ok(left < right),
        Opcode::LessEqualByte => Ok(left <= right),
        Opcode::GreaterEqualByte => Ok(left >= right),
        _ => Err(VmError::UnsupportedOpcode(opcode)),
    }
}
