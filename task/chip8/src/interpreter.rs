use core::ops::{Index, IndexMut};

use crate::{
    data::{Address, Nibble, OpCode, RegisterIndex, Word},
    image::Image,
    platform::{Platform, Point, Sprite},
    Error, Offset, Result,
};

////////////////////////////////////////////////////////////////////////////////

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

pub const FONT_ADDRESS: Address = Address::new(0x0);
pub const FONT_HEIGHT: Offset = 5;
pub const FONT_SPRITES: [u8; 16 * FONT_HEIGHT as usize] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

////////////////////////////////////////////////////////////////////////////////

pub struct Interpreter<P: Platform> {
    platform: P,
    // TODO: your code here.
}

impl<P: Platform> Interpreter<P> {
    pub fn new(image: impl Image, platform: P) -> Self {
        Self {
            platform,
            // TODO: your code here.
        }
    }

    pub fn platform(&self) -> &P {
        &self.platform
    }

    pub fn platform_mut(&mut self) -> &mut P {
        &mut self.platform
    }

    pub fn run_next_instruction(&mut self) -> Result<()> {
        // TODO: your code here.
        unimplemented!()
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy)]
pub enum Operation {
    ClearScreen,
    Return,
    Jump(Address),
    Call(Address),
    SkipIfEqual(RegisterIndex, Word),
    SkipIfNotEqual(RegisterIndex, Word),
    SkipIfRegistersEqual(RegisterIndex, RegisterIndex),
    SetRegister(RegisterIndex, Word),
    AddValue(RegisterIndex, Word),
    SetToRegister(RegisterIndex, RegisterIndex),
    Or(RegisterIndex, RegisterIndex),
    And(RegisterIndex, RegisterIndex),
    Xor(RegisterIndex, RegisterIndex),
    AddRegister(RegisterIndex, RegisterIndex),
    SubRegister(RegisterIndex, RegisterIndex),
    ShiftRight(RegisterIndex, RegisterIndex),
    SubRegisterReversed(RegisterIndex, RegisterIndex),
    ShiftLeft(RegisterIndex, RegisterIndex),
    SkipIfRegistersNotEqual(RegisterIndex, RegisterIndex),
    SetIndexRegister(Address),
    JumpV0(Address),
    SetToRandom(RegisterIndex, Word),
    Draw(RegisterIndex, RegisterIndex, Nibble),
    SkipIfKeyDown(RegisterIndex),
    SkipIfKeyUp(RegisterIndex),
    GetDelayTimer(RegisterIndex),
    WaitForKey(RegisterIndex),
    SetDelayTimer(RegisterIndex),
    SetSoundTimer(RegisterIndex),
    IncrementIndexRegister(RegisterIndex),
    SetIndexRegisterToSprite(Nibble),
    ToDecimal(RegisterIndex),
    WriteMemory(Nibble),
    ReadMemory(Nibble),
}

impl TryFrom<OpCode> for Operation {
    type Error = ();

    fn try_from(code: OpCode) -> core::result::Result<Self, ()> {
        let op = match code.as_u16() {
            // TODO: your code here.
            _ => return Err(()),
        };
        Ok(op)
    }
}

////////////////////////////////////////////////////////////////////////////////
