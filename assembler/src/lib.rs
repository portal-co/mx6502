#![no_std]
extern crate alloc;

use alloc::{collections::btree_map::BTreeMap, string::{String, ToString}, vec::Vec};
use portal_solutions_mos6502_model::*;


enum Data {
    LiteralByte(u8),
    LabelOffsetLe(String),
    LiteralOffsetLe(Address),
    LiteralAddressLe(Address),
    LabelOffsetLo(String),
    LabelOffsetHi(String),
    LabelRelativeOffset(String),
}

struct DataAtOffset {
    data: Data,
    offset: Address,
}

pub struct Block {
    cursor_offset: Address,
    program: Vec<DataAtOffset>,
    labels: BTreeMap<String, Address>,
}

pub trait ArgOperand {
    type Operand: operand::Trait;
    fn program(self, block: &mut Block);
}

impl ArgOperand for &'static str {
    type Operand = operand::Address;
    fn program(self, block: &mut Block) {
        block.label_offset_le(self);
    }
}

impl ArgOperand for String {
    type Operand = operand::Address;
    fn program(self, block: &mut Block) {
        block.label_offset_le(self);
    }
}

impl ArgOperand for Address {
    type Operand = operand::Address;
    fn program(self, block: &mut Block) {
        block.literal_address_le(self);
    }
}

impl ArgOperand for u8 {
    type Operand = operand::Byte;
    fn program(self, block: &mut Block) {
        block.literal_byte(self);
    }
}

impl ArgOperand for i8 {
    type Operand = operand::Byte;
    fn program(self, block: &mut Block) {
        block.literal_byte(self as u8);
    }
}

pub struct Addr(pub Address);

impl ArgOperand for Addr {
    type Operand = operand::Address;
    fn program(self, block: &mut Block) {
        block.literal_address_le(self.0);
    }
}

// Inside 6502 "assembly" programs, rust infers int literals to
// be i32 rather than u8. This treats i32 as u8 to prevent the
// need for explicit type coersion in assembly programs.
impl ArgOperand for i32 {
    type Operand = operand::Byte;
    fn program(self, block: &mut Block) {
        // Allow the union of signed and unsigned byte ranges. This is to
        // prevent mistakes such as writing 0x011011010 instead of 0b011011010.
        assert!(self >= -128 && self <= 255, "{} is not a valid byte", self);
        block.literal_byte((self as i8) as u8);
    }
}

impl ArgOperand for () {
    type Operand = operand::None;
    fn program(self, _block: &mut Block) {}
}

pub struct LabelOffsetLo(pub &'static str);
pub struct LabelOffsetHi(pub &'static str);
pub struct LabelRelativeOffset(pub &'static str);
pub struct LabelRelativeOffsetOwned(pub String);

impl ArgOperand for LabelOffsetLo {
    type Operand = operand::Byte;
    fn program(self, block: &mut Block) {
        block.label_offset_lo(self.0);
    }
}

impl ArgOperand for LabelOffsetHi {
    type Operand = operand::Byte;
    fn program(self, block: &mut Block) {
        block.label_offset_hi(self.0);
    }
}

impl ArgOperand for LabelRelativeOffset {
    type Operand = operand::Byte;
    fn program(self, block: &mut Block) {
        block.label_relative_offset(self.0);
    }
}

impl ArgOperand for LabelRelativeOffsetOwned {
    type Operand = operand::Byte;
    fn program(self, block: &mut Block) {
        block.label_relative_offset(self.0.as_str());
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    OffsetOutOfBounds,
    UndeclaredLabel(String),
    BranchTargetOutOfRange(String),
}

impl Block {
    pub fn new() -> Self {
        Self {
            cursor_offset: 0,
            program: Vec::new(),
            labels: BTreeMap::new(),
        }
    }
    pub fn set_offset(&mut self, offset: Address) {
        self.cursor_offset = offset;
    }
    pub fn literal_byte(&mut self, byte: u8) {
        self.program.push(DataAtOffset {
            data: Data::LiteralByte(byte),
            offset: self.cursor_offset,
        });
        self.cursor_offset = self.cursor_offset.wrapping_add(1);
    }
    pub fn literal_offset_le(&mut self, offset: Address) {
        self.program.push(DataAtOffset {
            data: Data::LiteralOffsetLe(offset),
            offset: self.cursor_offset,
        });
        self.cursor_offset = self.cursor_offset.wrapping_add(2);
    }
    pub fn literal_address_le(&mut self, offset: Address) {
        self.program.push(DataAtOffset {
            data: Data::LiteralAddressLe(offset),
            offset: self.cursor_offset,
        });
        self.cursor_offset = self.cursor_offset.wrapping_add(2);
    }
    pub fn label_offset_le<S: AsRef<str>>(&mut self, label: S) {
        let string = label.as_ref().to_string();
        self.program.push(DataAtOffset {
            data: Data::LabelOffsetLe(string),
            offset: self.cursor_offset,
        });
        self.cursor_offset = self.cursor_offset.wrapping_add(2);
    }
    pub fn label_offset_lo<S: AsRef<str>>(&mut self, label: S) {
        let string = label.as_ref().to_string();
        self.program.push(DataAtOffset {
            data: Data::LabelOffsetLo(string),
            offset: self.cursor_offset,
        });
        self.cursor_offset = self.cursor_offset.wrapping_add(1);
    }
    pub fn label_offset_hi<S: AsRef<str>>(&mut self, label: S) {
        let string = label.as_ref().to_string();
        self.program.push(DataAtOffset {
            data: Data::LabelOffsetHi(string),
            offset: self.cursor_offset,
        });
        self.cursor_offset = self.cursor_offset.wrapping_add(1);
    }
    pub fn label_relative_offset<S: AsRef<str>>(&mut self, label: S) {
        let string = label.as_ref().to_string();
        self.program.push(DataAtOffset {
            data: Data::LabelRelativeOffset(string),
            offset: self.cursor_offset,
        });
        self.cursor_offset = self.cursor_offset.wrapping_add(1);
    }
    pub fn label<S: AsRef<str>>(&mut self, s: S) {
        let string = s.as_ref().to_string();
        if self.labels.insert(string, self.cursor_offset).is_some() {
            panic!("Multiple definitions of label {}", s.as_ref());
        }
    }
    pub fn inst<
        I: AssemblerInstruction,
        A: ArgOperand<Operand = <I::AddressingMode as addressing_mode::Trait>::Operand>,
    >(
        &mut self,
        instruction: I,
        arg: A,
    ) {
        let _ = instruction;
        self.literal_byte(I::opcode());
        arg.program(self);
    }
    pub fn infinite_loop(&mut self) {
        let offset = self.cursor_offset;
        self.literal_byte(assembler_instruction::Jmp::<addressing_mode::Absolute>::opcode());
        self.literal_offset_le(offset);
    }
    pub fn assemble(
        &self,
        base: Address,
        size: usize,
        buffer: &mut Vec<u8>,
    ) -> Result<AssembledBlock, Error> {
        let mut labels = BTreeMap::new();
        for (label, address) in self.labels.iter() {
            labels.insert(label.clone(), address + base);
        }
        buffer.resize(size, 0);
        for &DataAtOffset { offset, ref data } in self.program.iter() {
            match data {
                &Data::LiteralByte(byte) => {
                    if offset as usize >= size {
                        return Err(Error::OffsetOutOfBounds);
                    }
                    buffer[offset as usize] = byte;
                }
                Data::LabelOffsetLe(label) => {
                    if let Some(&label_offset) = self.labels.get(label) {
                        if offset as usize + 1 >= size {
                            return Err(Error::OffsetOutOfBounds);
                        }
                        let address = label_offset + base;
                        buffer[offset as usize] = address::lo(address);
                        buffer[offset as usize + 1] = address::hi(address);
                    } else {
                        return Err(Error::UndeclaredLabel(label.clone()));
                    }
                }
                Data::LiteralOffsetLe(literal_offset) => {
                    if offset as usize + 1 >= size {
                        return Err(Error::OffsetOutOfBounds);
                    }
                    let address = literal_offset + base;
                    buffer[offset as usize] = address::lo(address);
                    buffer[offset as usize + 1] = address::hi(address);
                }
                &Data::LiteralAddressLe(address) => {
                    buffer[offset as usize] = address::lo(address);
                    buffer[offset as usize + 1] = address::hi(address);
                }
                Data::LabelOffsetLo(label) => {
                    if let Some(&label_offset) = self.labels.get(label) {
                        if offset as usize + 1 >= size {
                            return Err(Error::OffsetOutOfBounds);
                        }
                        let address = label_offset + base;
                        buffer[offset as usize] = address::lo(address);
                    } else {
                        return Err(Error::UndeclaredLabel(label.clone()));
                    }
                }
                Data::LabelOffsetHi(label) => {
                    if let Some(&label_offset) = self.labels.get(label) {
                        if offset as usize + 1 >= size {
                            return Err(Error::OffsetOutOfBounds);
                        }
                        let address = label_offset + base;
                        buffer[offset as usize] = address::hi(address);
                    } else {
                        return Err(Error::UndeclaredLabel(label.clone()));
                    }
                }
                Data::LabelRelativeOffset(label) => {
                    if let Some(&label_offset) = self.labels.get(label) {
                        let delta = label_offset as i16 - offset as i16 - 1;
                        if delta < -128 || delta > 127 {
                            return Err(Error::BranchTargetOutOfRange(label.clone()));
                        }
                        buffer[offset as usize] = (delta as i8) as u8;
                    } else {
                        return Err(Error::UndeclaredLabel(label.clone()));
                    }
                }
            }
        }
        Ok(AssembledBlock { labels })
    }
}

pub struct AssembledBlock {
    labels: BTreeMap<String, Address>,
}

impl AssembledBlock {
    pub fn address_of_label(&self, label: &str) -> Option<Address> {
        self.labels.get(label).cloned()
    }
}
