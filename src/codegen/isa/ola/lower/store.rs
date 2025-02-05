use super::{get_inst_output, get_operand_for_val, new_empty_inst_output};
use crate::codegen::core::ir::{
    function::instruction::{InstructionId, Opcode as IrOpcode},
    types::Type,
    value::{ConstantInt, ConstantValue, Value, ValueId},
};
use crate::codegen::{
    function::instruction::Instruction as MachInstruction,
    isa::ola::{
        instruction::{InstructionData, Opcode, Operand as MOperand, OperandData},
        register::GR,
        Ola,
    },
    lower::{LoweringContext, LoweringError},
    register::Reg,
};
use anyhow::Result;

pub fn lower_store(
    ctx: &mut LoweringContext<Ola>,
    tys: &[Type],
    args: &[ValueId],
    _align: u32,
) -> Result<()> {
    let mut dst_slot = None;
    let mut dst_vreg = None;

    let dst = args[1];
    match ctx.ir_data.value_ref(dst) {
        Value::Instruction(id) => {
            // Maybe Alloca
            if let Some(slot_id) = ctx.inst_id_to_slot_id.get(id) {
                dst_slot = Some(*slot_id);
            } else if ctx.ir_data.instructions[*id].opcode == IrOpcode::GetElementPtr {
                return lower_store_gep(ctx, tys, args, _align, *id);
            } else {
                dst_vreg = Some(get_inst_output(ctx, tys[1], *id)?);
            }
        }
        _ => {
            return Err(
                LoweringError::Todo("Store dest must be an instruction result".into()).into(),
            )
        }
    }

    let konst = None;
    let vreg;

    let src = args[0];
    println!(
        "store src {:?}, src type {:?}",
        ctx.ir_data.value_ref(src),
        tys[0]
    );
    match ctx.ir_data.value_ref(src) {
        Value::Constant(c) => {
            let addr = ctx.mach_data.vregs.add_vreg_data(tys[0]);
            match c {
                ConstantValue::Int(ConstantInt::Int32(i)) => {
                    //let addr = ctx.mach_data.vregs.add_vreg_data(tys[0]);
                    ctx.inst_seq.push(MachInstruction::new(
                        InstructionData {
                            opcode: Opcode::MOVri,
                            operands: vec![
                                MOperand::output(addr.into()),
                                MOperand::new(OperandData::Int32(*i)),
                            ],
                        },
                        ctx.block_map[&ctx.cur_block],
                    ));
                    //vreg = Some(addr.into());
                }
                e => {
                    return Err(
                        LoweringError::Todo(format!("Unsupported store source: {:?}", e)).into(),
                    )
                }
            }
            vreg = Some(addr.into());
        }
        Value::Instruction(id) => {
            if tys[0].is_pointer(ctx.types) {
                let addr = ctx.mach_data.vregs.add_vreg_data(tys[0]);
                let inst = ctx.ir_data.inst_ref(*id);
                println!("store pointer opcode {:?}", inst.opcode);
                if inst.opcode == IrOpcode::Alloca {
                    let fp: Reg = GR::R8.into();
                    let mut src_slot = None;
                    if let Some(slot_id) = ctx.inst_id_to_slot_id.get(id) {
                        src_slot = Some(*slot_id);
                    }
                    println!("store pointer opcode {:?},slot {:?}", inst.opcode, src_slot);
                    ctx.inst_seq.push(MachInstruction::new(
                        InstructionData {
                            opcode: Opcode::ADDri,
                            operands: vec![
                                MOperand::output(addr.into()),
                                MOperand::new(fp.into()),
                                MOperand::new(OperandData::Slot(src_slot.unwrap())),
                            ],
                        },
                        ctx.block_map[&ctx.cur_block],
                    ));
                }
                // vreg = Some(addr.into());
                // vreg = Some(get_inst_output(ctx, tys[0], *id)?);
                vreg = Some(new_empty_inst_output(ctx, tys[0], *id).into());
            } else {
                vreg = Some(get_inst_output(ctx, tys[0], *id)?);
            }
        }
        Value::Argument(a) => vreg = ctx.arg_idx_to_vreg.get(&a.nth).copied(),
        e => return Err(LoweringError::Todo(format!("Unsupported store source: {:?}", e)).into()),
    }

    match (dst_vreg, dst_slot, vreg, konst) {
        (None, Some(slot), Some(vreg), None) => {
            ctx.inst_seq.append(&mut vec![MachInstruction::new(
                InstructionData {
                    opcode: Opcode::MSTOREr,
                    operands: vec![
                        MOperand::new(OperandData::MemStart),
                        MOperand::new(OperandData::None),
                        MOperand::new(OperandData::Slot(slot)),
                        MOperand::new(OperandData::None),
                        MOperand::input(OperandData::None),
                        MOperand::input(OperandData::None),
                        MOperand::new(OperandData::None),
                        MOperand::input(vreg.into()),
                    ],
                },
                ctx.block_map[&ctx.cur_block],
            )]);
            Ok(())
        }
        (Some(dst), None, Some(vreg), None) => {
            ctx.inst_seq.append(&mut vec![MachInstruction::new(
                InstructionData {
                    opcode: Opcode::MSTOREr,
                    operands: vec![
                        MOperand::new(OperandData::MemStart),
                        MOperand::new(OperandData::None),
                        MOperand::new(OperandData::None),
                        MOperand::new(OperandData::None),
                        MOperand::input(dst.into()),
                        MOperand::input(OperandData::None),
                        MOperand::new(OperandData::None),
                        MOperand::input(vreg.into()),
                    ],
                },
                ctx.block_map[&ctx.cur_block],
            )]);
            Ok(())
        }
        (None, Some(slot), None, Some(konst)) => {
            ctx.inst_seq.append(&mut vec![MachInstruction::new(
                InstructionData {
                    opcode: Opcode::MSTOREi,
                    operands: vec![
                        MOperand::new(OperandData::MemStart),
                        MOperand::new(OperandData::None),
                        MOperand::new(OperandData::Slot(slot)),
                        MOperand::new(OperandData::None),
                        MOperand::input(OperandData::None),
                        MOperand::input(OperandData::None),
                        MOperand::new(OperandData::None),
                        MOperand::input(match konst {
                            ConstantValue::Int(ConstantInt::Int8(i)) => i.into(),
                            ConstantValue::Int(ConstantInt::Int32(i)) => i.into(),
                            ConstantValue::Null(_) => 0i64.into(),
                            _ => panic!(),
                        }),
                    ],
                },
                ctx.block_map[&ctx.cur_block],
            )]);
            Ok(())
        }
        e => Err(LoweringError::Todo(format!("Unsupported store dest pattern: {:?}", e)).into()),
    }
}

fn lower_store_gep(
    ctx: &mut LoweringContext<Ola>,
    tys: &[Type],
    args: &[ValueId],
    _align: u32,
    gep_id: InstructionId,
) -> Result<()> {
    use crate::codegen::isa::TargetIsa;
    use {
        Constant as Const,
        ConstantInt::{Int32, Int64},
        ConstantValue::Int,
        Value::Constant,
    };

    let gep = &ctx.ir_data.instructions[gep_id];
    let gep_args: Vec<&Value> = gep
        .operand
        .args()
        .iter()
        .map(|&arg| &ctx.ir_data.values[arg])
        .collect();

    let mem = match &gep_args[..] {
        [Value::Instruction(base_ptr), Const(Int(Int64(idx0))), Const(Int(Int64(idx1)))] => {
            let base_ptr = ctx.inst_id_to_slot_id[base_ptr];
            let base_ty = gep.operand.types()[0];
            let offset = idx0 * ctx.isa.data_layout().get_size_of(ctx.types, base_ty) as i64
                + idx1
                    * ctx
                        .isa
                        .data_layout()
                        .get_size_of(ctx.types, ctx.types.get_element(base_ty).unwrap())
                        as i64;

            vec![
                MOperand::new(OperandData::MemStart),
                MOperand::new(OperandData::None),
                MOperand::new(OperandData::Slot(base_ptr)),
                MOperand::new(OperandData::Int32(-offset as i32)),
                MOperand::input(OperandData::None),
                MOperand::input(OperandData::None),
                MOperand::new(OperandData::None),
            ]
        }
        [Value::Instruction(base_ptr), Const(Int(Int32(idx0))), Const(Int(Int32(idx1)))] => {
            let base_ptr = ctx.inst_id_to_slot_id[base_ptr];
            let base_ty = gep.operand.types()[0];
            let offset = idx0 * ctx.isa.data_layout().get_size_of(ctx.types, base_ty) as i32
                + idx1
                    * ctx
                        .isa
                        .data_layout()
                        .get_size_of(ctx.types, ctx.types.get_element(base_ty).unwrap())
                        as i32;
            vec![
                MOperand::new(OperandData::MemStart),
                MOperand::new(OperandData::None),
                MOperand::new(OperandData::Slot(base_ptr)),
                MOperand::new(OperandData::Int32(-offset as i32)),
                MOperand::input(OperandData::None),
                MOperand::input(OperandData::None),
                MOperand::new(OperandData::None),
            ]
        }
        [Value::Instruction(base_ptr), Const(Int(Int64(idx0))), Value::Instruction(idx1)] => {
            let mut slot = None;
            let mut base = None;
            if let Some(p) = ctx.inst_id_to_slot_id.get(base_ptr) {
                slot = Some(*p);
            } else {
                base = Some(get_operand_for_val(
                    ctx,
                    gep.operand.types()[1],
                    gep.operand.args()[0],
                )?);
            }

            let base_ty = gep.operand.types()[0];
            let offset = idx0 * ctx.isa.data_layout().get_size_of(ctx.types, base_ty) as i64;

            let idx1_ty = gep.operand.types()[3];
            assert!(idx1_ty.is_i64());
            let idx1 = get_inst_output(ctx, idx1_ty, *idx1)?;

            assert!(
                ctx.isa
                    .data_layout()
                    .get_size_of(ctx.types, ctx.types.get_element(base_ty).unwrap())
                    == 4
            );

            vec![
                MOperand::new(OperandData::MemStart),
                MOperand::new(OperandData::None),
                MOperand::new(slot.map_or(OperandData::None, |s| OperandData::Slot(s))),
                MOperand::new(OperandData::Int32(-offset as i32)),
                MOperand::input(base.map_or(OperandData::None, |x| x)),
                MOperand::input(OperandData::VReg(idx1)),
                MOperand::new(OperandData::Int32(
                    ctx.isa
                        .data_layout()
                        .get_size_of(ctx.types, ctx.types.get_element(base_ty).unwrap())
                        as i32,
                )),
            ]
        }
        e => {
            return Err(
                LoweringError::Todo(format!("Unsupported GEP pattern for store: {:?}", e)).into(),
            )
        }
    };

    ctx.mark_as_merged(gep_id);

    let src = args[0];
    let src_ty = tys[0];
    match ctx.ir_data.value_ref(src) {
        Const(Int(Int32(int))) => {
            let addr = ctx.mach_data.vregs.add_vreg_data(tys[0]);
            ctx.inst_seq.push(MachInstruction::new(
                InstructionData {
                    opcode: Opcode::MOVri,
                    operands: vec![
                        MOperand::output(addr.into()),
                        MOperand::new(OperandData::Int32(*int)),
                    ],
                },
                ctx.block_map[&ctx.cur_block],
            ));
            ctx.inst_seq.append(&mut vec![MachInstruction::new(
                InstructionData {
                    opcode: Opcode::MSTOREr,
                    operands: mem
                        .into_iter()
                        .chain(vec![MOperand::input(addr.into())].into_iter())
                        .collect(),
                },
                ctx.block_map[&ctx.cur_block],
            )]);
        }
        Value::Instruction(id) => {
            let src = get_inst_output(ctx, src_ty, *id)?;
            ctx.inst_seq.append(&mut vec![MachInstruction::new(
                InstructionData {
                    opcode: Opcode::MSTOREr,
                    operands: mem
                        .into_iter()
                        .chain(vec![MOperand::input(src.into())].into_iter())
                        .collect(),
                },
                ctx.block_map[&ctx.cur_block],
            )]);
        }
        e => {
            return Err(LoweringError::Todo(format!("Unsupported store source 2: {:?}", e)).into())
        }
    }

    Ok(())
}
