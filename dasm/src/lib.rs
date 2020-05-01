use std::io;
use std::io::prelude::*;
use std::num::Wrapping;

const DEFAULT_ORIGIN: u16 = 0x4000;

pub struct DasmOptions {
  pub starting_address: Option<u16>,
}

pub fn disassemble<R, W>(mut input: R, mut output: W, options: DasmOptions) -> io::Result<()>
  where
    R: Read,
    W: Write,
{
  let pc = options.starting_address.unwrap_or(DEFAULT_ORIGIN);
  let mut byte = [0u8; 1];

  while input.read(&mut byte)? != 0 {
  }

  Ok(())
}

struct Instruction {
  name: &'static str,
  addr_mode: AddressMode,
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
#[repr(u32)]
enum AddressMode {
  /// Accumulator
  Accum = 0,
  /// Absolute
  Abs,
  /// Absolute, X indexed
  AbsX,
  /// Absolute, Y indexed
  AbsY,
  /// Immediate
  Imm,
  /// Implied
  Impl,
  /// Indirect
  Ind,
  /// X-indexed, indirect
  XInd,
  /// Indirect, Y-indexed
  IndY,
  /// Relative
  Rel,
  /// ZeroPage
  Zpg,
  /// ZeroPage, X-indexed
  ZpgX,
  /// ZeroPage, 
  ZpgY,
  NumOfAddressModes,
}

macro_rules! inst {
  ($name:literal $mode:ident) => {
    Instruction::new($name, AddressMode::$mode)
  };
}

/// Data from <https://www.masswerk.at/6502/6502_instruction_set.html>.
static INSTRUCTION_TABLE: [Option<Instruction>; 256] = [
  // 00-0f
  inst!("INT" Abs),
  inst!("ORA" XInd),
  None,
  None,
  None,
  inst!("ORA" Zpg),
  inst!("ASL" Zpg),
  None,
  inst!("PHP" Impl),
  inst!("ORA" Imm),
  inst!("ASL" Accum),
  None,
  None,
  inst!("ORA" Abs),
  inst!("ASL" Abs),
  None,

  // 10-1f
  inst!("BPL" Rel),
  inst!("ORA" IndY),
  None,
  None,
  None,
  inst!("ORA" ZpgX),
  inst!("ASL" ZpgX),
  None,
  inst!("CLC" Impl),
  inst!("ORA" AbsY),
  None,
  None,
  None,
  inst!("ORA" AbsX),
  inst!("ASL" AbsX),
  None,

  // 20-2f
  inst!("JSR" Abs),
  inst!("AND" XInd),
  None,
  None,
  inst!("BIT" Zpg),
  inst!("AND" Zpg),
  inst!("ROL" Zpg),
  None,
  inst!("PLP" Impl),
  inst!("AND" Imm),
  inst!("ROL" Accum),
  None,
  inst!("BIT" Abs),
  inst!("AND" Abs),
  inst!("ROL" Abs),
  None,

  // 30-3f
  inst!("BMI" Rel),
  inst!("AND" IndY),
  None,
  None,
  None,
  inst!("AND" ZpgX),
  inst!("ROL" ZpgX),
  None,
  inst!("SEC" Impl),
  inst!("AND" AbsY),
  None,
  None,
  None,
  inst!("AND" AbsX),
  inst!("ROL" AbsX),
  None,

  // 40-4f
  inst!("RTI" Impl),
  inst!("EOR" XInd),
  None,
  None,
  None,
  inst!("EOR" Zpg),
  inst!("LSR" Zpg),
  None,
  inst!("PHA" Impl),
  inst!("EOR" Imm),
  inst!("LSR" Accum),
  None,
  inst!("JMP" Abs),
  inst!("EOR" Abs),
  inst!("LSR" Abs),
  None,

  // 50-5f
  inst!("BVC" Rel),
  inst!("EOR" IndY),
  None,
  None,
  None,
  inst!("EOR" ZpgX),
  inst!("LSR" ZpgX),
  None,
  inst!("CLI" Impl),
  inst!("EOR" AbsY),
  None,
  None,
  None,
  inst!("EOR" AbsX),
  inst!("LSR" AbsX),
  None,

  // 60-6f
  inst!("RTS" Impl),
  inst!("ADC" XInd),
  None,
  None,
  None,
  inst!("ADC" Zpg),
  inst!("ROR" Zpg),
  None,
  inst!("PLA" Impl),
  inst!("ADC" Imm),
  inst!("ROR" Accum),
  None,
  inst!("JMP" Ind),
  inst!("ADC" Abs),
  inst!("ROR" Abs),
  None,

  // 70-7f
  inst!("BVS" Rel),
  inst!("ADC" IndY),
  None,
  None,
  None,
  inst!("ADC" ZpgX),
  inst!("ROR" ZpgX),
  None,
  inst!("SEI" Impl),
  inst!("ADC" AbsY),
  None,
  None,
  None,
  inst!("ADC" AbsX),
  inst!("ROR" AbsX),
  None,

  // 80-8f
  None,
  inst!("STA" XInd),
  None,
  None,
  inst!("STY" Zpg),
  inst!("STA" Zpg),
  inst!("STX" Zpg),
  None,
  inst!("DEY" Impl),
  None,
  inst!("TXA" Impl),
  None,
  inst!("STY" Abs),
  inst!("STA" Abs),
  inst!("STX" Abs),
  None,

  // 90-9f
  inst!("BCC" Rel),
  inst!("STA" IndY),
  None,
  None,
  inst!("STY" ZpgX),
  inst!("STA" ZpgX),
  inst!("STX" ZpgY),
  None,
  inst!("TYA" Impl),
  inst!("STA" AbsY),
  inst!("TXA" Impl),
  None,
  None,
  inst!("STA" AbsX),
  None,
  None,

  // a0-af
  inst!("LDY" Imm),
  inst!("LDA" XInd),
  inst!("LDX" Imm),
  None,
  inst!("LDY" Zpg),
  inst!("LDA" Zpg),
  inst!("LDX" Zpg),
  None,
  inst!("TAY" Impl),
  inst!("LDA" Imm),
  inst!("TAX" Impl),
  None,
  inst!("LDY" Abs),
  inst!("LDA" Abs),
  inst!("LDX" Abs),
  None,

  // b0-bf
  inst!("BCS" Rel),
  inst!("LDA" IndY),
  None,
  None,
  inst!("LDY" ZpgX),
  inst!("LDA" ZpgX),
  inst!("LDX" ZpgY),
  None,
  inst!("CLV" Impl),
  inst!("LDA" AbsY),
  inst!("TSX" Impl),
  None,
  inst!("LDY" AbsX),
  inst!("LDA" AbsX),
  inst!("LDX" AbsY),
  None,

  // c0-cf
  inst!("CPY" Imm),
  inst!("CMP" XInd),
  None,
  None,
  inst!("CPY" Zpg),
  inst!("CMP" Zpg),
  inst!("DEC" Zpg),
  None,
  inst!("INY" Impl),
  inst!("CMP" Imm),
  inst!("DEX" Impl),
  None,
  inst!("CPY" Abs),
  inst!("CMP" Abs),
  inst!("DEC" Abs),
  None,

  // d0-df
  inst!("BNE" Rel),
  inst!("CMP" IndY),
  None,
  None,
  None,
  inst!("CMP" ZpgX),
  inst!("DEC" ZpgX),
  None,
  inst!("CLD" Impl),
  inst!("CMP" AbsY),
  None,
  None,
  None,
  inst!("CMP" AbsX),
  inst!("DEC" AbsX),
  None,

  // e0-ef
  inst!("CPX" Imm),
  inst!("SBC" XInd),
  None,
  None,
  inst!("CPX" Zpg),
  inst!("SBC" Zpg),
  inst!("INC" Zpg),
  None,
  inst!("INX" Impl),
  inst!("SBC" Imm),
  inst!("NOP" Impl),
  None,
  inst!("CPX" Abs),
  inst!("SBC" Abs),
  inst!("INC" Abs),
  None,
  
  // f0-ff
  inst!("BEQ" Rel),
  inst!("SBC" IndY),
  None,
  None,
  None,
  inst!("SBC" ZpgX),
  inst!("INC" ZpgX),
  None,
  inst!("SED" Impl),
  inst!("SBC" AbsY),
  None,
  None,
  None,
  inst!("SBC" AbsX),
  inst!("INC" AbsX),
  None,
];


impl AddressMode {
  fn instruction_size(self) -> usize {
    use AddressMode::*;

    match self {
      Accum => 1,
      Abs => 3,
      AbsX => 3,
      AbsY => 3,
      Imm => 2,
      Impl => 1,
      Ind => 3,
      XInd => 2,
      IndY => 2,
      Rel => 3,
      Zpg => 2,
      ZpgX => 2,
      ZpgY => 2,
      NumOfAddressModes => unreachable!()
    }
  }

  fn write<W: Write>(self, pc: u16, operand: &[u8], w: &mut W) -> io::Result<()> {
    use AddressMode::*;

    match self {
      Accum => Ok(()),
      Abs => write!(w, " ${:02X}{:02X}", operand[1], operand[0]),
      AbsX => write!(w, " ${:02X}{:02X},X", operand[1], operand[0]),
      AbsY => write!(w, " ${:02X}{:02X},Y", operand[1], operand[0]),
      Imm => write!(w, " #${:02X}", operand[0]),
      Impl => Ok(()),
      Ind => write!(w, " (${:02X}{:02X})", operand[1], operand[0]),
      XInd => write!(w, " (${:02X},X)", operand[0]),
      IndY => write!(w, " (${:02X}),Y", operand[0]),
      Rel => write!(w, " ${:04X}", (Wrapping(pc) + Wrapping(operand[0] as i8 as u16)).0),
      Zpg => write!(w, " ${:02X}", operand[0]),
      ZpgX => write!(w, " ${:02X},X", operand[0]),
      ZpgY => write!(w, " ${:02X},Y", operand[0]),
      NumOfAddressModes => unreachable!(),
    }
  }
}

impl Instruction {
  const fn new(name: &'static str, addr_mode: AddressMode) -> Option<Self> {
    Some(Self { name, addr_mode })
  }
}