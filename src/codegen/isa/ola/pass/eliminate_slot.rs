use crate::codegen::core::ir::types;
use crate::codegen::{
    function::{instruction::TargetInst, Function},
    isa::ola::{instruction::Opcode, instruction::OperandData, register::GR, Ola},
    module::Module,
};
use anyhow::Result;

pub fn run_on_module(module: &mut Module<Ola>) -> Result<()> {
    for (_, func) in &mut module.functions {
        run_on_function(func);
    }
    Ok(())
}

pub fn run_on_function(function: &mut Function<Ola>) {
    let mut worklist = vec![];
    let mut call = false;

    for block in function.layout.block_iter() {
        for inst_id in function.layout.inst_iter(block) {
            let inst = function.data.inst_ref(inst_id);
            if inst
                .data
                .operands
                .iter()
                .any(|op| matches!(op.data, OperandData::Slot(_)))
            {
                println!("slot worklist {:?}", inst.data.opcode);
                worklist.push(inst_id);
            }
            if inst.data.is_call() {
                call = true;
            }
        }
    }

    if call {
        for _ in 0..2 {
            function.slots.add_slot(types::I32, 4, 4);
        }
    }
    function.slots.ensure_computed_offsets();

    while let Some(inst_id) = worklist.pop() {
        let mut inst = function.data.instructions[inst_id].clone();

        let mut i = 0;
        let len = inst.data.operands.len();

        if inst.data.opcode == Opcode::ADDri
            && matches!(&inst.data.operands[len - 1].data, &OperandData::Slot(_))
        {
            println!("add slot {:?}", inst.data.operands[len - 1].data);
            match &inst.data.operands[len - 1].data {
                OperandData::Slot(slot) => {
                    let off = function.slots.get(*slot).offset;
                    let mut size = off as i32 / 4;
                    if call {
                        size = -size - 2;
                    }
                    inst.data.operands[len - 1].data = OperandData::Int32(size);
                    function.data.instructions[inst_id] = inst;
                }
                _ => todo!(),
            }
            continue;
        }

        while i < len {
            // MemStart indicates the beginning of memory arguments
            if !matches!(&inst.data.operands[i].data, &OperandData::MemStart) {
                i += 1;
                continue;
            }

            i += 1;

            let mem = &mut inst.data.operands[i..i + 6];

            match (&mem[1].data, &mem[2].data) {
                (OperandData::Slot(slot), OperandData::None) => {
                    let off = function.slots.get(*slot).offset;
                    mem[1].data = OperandData::None;
                    //let size = off as i32 / 4 - 1 - function.slots.arena.len() as i32;
                    let mut size = (0 - off as i32) / 4;
                    if call {
                        size -= 2;
                    }
                    println!(
                        "slot pattern 1 len: {:?}, off: {:?}, size: {:?}",
                        function.slots.arena.len() as i32,
                        off,
                        size
                    );
                    mem[2].data = OperandData::Int32(size);
                    mem[3].data = OperandData::Reg(GR::R8.into());
                }
                (OperandData::Slot(slot), OperandData::Int32(imm)) => {
                    let off = function.slots.get(*slot).offset;
                    // let size = function.slots.arena.len() as i32 + (*imm - off as i32) / 4 + 1;
                    let mut size = (-*imm - off as i32) / 4;
                    if call {
                        size -= 2;
                    }
                    println!(
                        "slot pattern 2 len: {:?}, off: {:?}, size: {:?}, imm {:?}",
                        function.slots.arena.len() as i32,
                        off,
                        size,
                        *imm
                    );
                    mem[2].data = OperandData::Int32(size);
                    mem[1].data = OperandData::None;
                    mem[3].data = OperandData::Reg(GR::R8.into());
                }
                (OperandData::Slot(slot), OperandData::Int64(imm)) => {
                    let off = function.slots.get(*slot).offset;
                    let size = function.slots.arena.len() as i64 + (*imm - off as i64) / 4 + 1;
                    println!(
                        "slot pattern 3 len: {:?}, off: {:?}, size: {:?}",
                        function.slots.arena.len() as i32,
                        off,
                        size
                    );
                    mem[2].data = OperandData::Int64(size);
                    mem[1].data = OperandData::None;
                    mem[3].data = OperandData::Reg(GR::R8.into());
                }
                _ => todo!(),
            }

            i += 6;
        }

        function.data.instructions[inst_id] = inst;
    }
}
