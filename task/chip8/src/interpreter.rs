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

pub const ENTRY_POINT_ADDRESS: Address = Address::new(0x200);

pub const REGISTERS_AMOUNT: usize = 16;

////////////////////////////////////////////////////////////////////////////////

pub struct Interpreter<P: Platform> {
    platform: P,
    index_register: Address,
    registers: Registers,
    memory: Memory,
}

impl<P: Platform> Interpreter<P> {
    pub fn new(image: impl Image, platform: P) -> Self {
        let mut memory = Memory::default();
        image.load_into_memory(&mut memory.locations);

        Self {
            platform,
            index_register: Address::new(0),
            registers: Registers::new(),
            memory,
        }
    }

    pub fn platform(&self) -> &P {
        &self.platform
    }

    pub fn platform_mut(&mut self) -> &mut P {
        &mut self.platform
    }

    pub fn run_next_instruction(&mut self) -> Result<()> {
        let opcode = self.memory.get_next_opcode();

        let operation = Operation::try_from(opcode)?;

        match operation {
            Operation::ClearScreen => self.clear_screen(),
            Operation::Jump(address) => self.jump(address),
            Operation::SetRegister(register_index, word) => self.set_register(register_index, word),
            Operation::SetIndexRegister(address) => self.index_register = address,
            Operation::Draw(x, y, n) => self.draw(x, y, n),
            _ => todo!(),
        }

        Ok(())
    }

    fn clear_screen(&mut self) {
        self.platform.clear_screen()
    }

    fn jump(&mut self, address: Address) {
        self.memory.instruction_pointer = address
    }

    fn set_register(&mut self, register_index: Nibble, word: u8) {
        self.registers.set(register_index, word)
    }

    fn draw(&mut self, x: Nibble, y: Nibble, n: Nibble) {
        let point = Point {
            x: self.registers.get(x),
            y: self.registers.get(y),
        };

        let sprite = Sprite::new(self.memory.get_slice(self.index_register, n.as_usize()));

        let had_pixels_flipped = self.platform.draw_sprite(point, sprite);
        self.set_register_f(had_pixels_flipped);
    }

    fn set_register_f(&mut self, value: bool) {
        const REG_F: RegisterIndex = Nibble(15);
        self.registers.set(REG_F, value as Word);
    }
}

pub struct Registers {
    words: [Word; REGISTERS_AMOUNT],
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}

impl Registers {
    pub fn new() -> Self {
        Self {
            words: [0; REGISTERS_AMOUNT],
        }
    }

    fn get(&self, index: RegisterIndex) -> Word {
        self.words[index.as_usize()]
    }

    fn set(&mut self, index: RegisterIndex, word: Word) {
        self.words[index.as_usize()] = word
    }
}

pub struct Memory {
    locations: [u8; Address::DOMAIN_SIZE],
    instruction_pointer: Address,
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl Memory {
    fn new() -> Self {
        Self {
            locations: [0; Address::DOMAIN_SIZE],
            instruction_pointer: ENTRY_POINT_ADDRESS,
        }
    }

    fn get_next_opcode(&mut self) -> OpCode {
        let ipa = self.instruction_pointer.as_usize();
        self.instruction_pointer += 2;

        OpCode::from_bytes(self.locations[ipa], self.locations[ipa + 1])
    }

    fn get_slice(&self, start: Address, size: usize) -> &[u8] {
        &self.locations[start.as_usize()..start.as_usize() + size]
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy)]
pub enum Operation {
    ClearScreen, // 00E0
    Return,
    Jump(Address), // 1nnn
    Call(Address),
    SkipIfEqual(RegisterIndex, Word),
    SkipIfNotEqual(RegisterIndex, Word),
    SkipIfRegistersEqual(RegisterIndex, RegisterIndex),
    SetRegister(RegisterIndex, Word), // 6xnn
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
    SetIndexRegister(Address), // Annn
    JumpV0(Address),
    SetToRandom(RegisterIndex, Word),
    Draw(RegisterIndex, RegisterIndex, Nibble), // Dxyn
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
    type Error = Error;

    fn try_from(code: OpCode) -> core::result::Result<Self, Error> {
        let error = Err(Error::UnknownOpCode(code));

        let op = match code.as_u16() {
            0x00e0 => Self::ClearScreen,
            _ => match code.extract_nibble(0).as_u8() {
                0x1 => Operation::Jump(code.extract_address()),
                0x6 => Operation::SetRegister(code.extract_nibble(1), code.extract_word(1)),
                0xa => Self::SetIndexRegister(code.extract_address()),
                0xd => Self::Draw(
                    code.extract_nibble(1),
                    code.extract_nibble(2),
                    code.extract_nibble(3),
                ),
                _ => return error,
            },
        };

        Ok(op)
    }
}

////////////////////////////////////////////////////////////////////////////////
