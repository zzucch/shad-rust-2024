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
pub const STACK_SIZE: usize = 16;

////////////////////////////////////////////////////////////////////////////////

pub struct Interpreter<P: Platform> {
    platform: P,
    index_register: Address,
    registers: Registers,
    memory: Memory,
    stack: Stack,
}

impl<P: Platform> Interpreter<P> {
    pub fn new(image: impl Image, platform: P) -> Self {
        let stack = Stack::default();

        let mut memory = Memory::default();
        image.load_into_memory(&mut memory.locations);

        Self {
            platform,
            index_register: Address::new(0),
            registers: Registers::new(),
            memory,
            stack,
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
            Operation::AddValue(register_index, word) => self.add_value(register_index, word),
            Operation::SkipIfEqual(register_index, word) => {
                self.skip_if_equal(register_index, word)
            }
            Operation::SkipIfNotEqual(register_index, word) => {
                self.skip_if_not_equal(register_index, word)
            }
            Operation::SkipIfRegistersEqual(register_index_first, register_index_second) => {
                self.skip_if_registers_equal(register_index_first, register_index_second)
            }
            Operation::SkipIfRegistersNotEqual(register_index_first, register_index_second) => {
                self.skip_if_registers_not_equal(register_index_first, register_index_second)
            }
            Operation::SetToRegister(register_index_first, register_index_second) => {
                self.set_to_register(register_index_first, register_index_second)
            }
            Operation::Or(register_index_first, register_index_second) => {
                self.or(register_index_first, register_index_second)
            }
            Operation::And(register_index_first, register_index_second) => {
                self.and(register_index_first, register_index_second)
            }
            Operation::Xor(register_index_first, register_index_second) => {
                self.xor(register_index_first, register_index_second)
            }
            Operation::AddRegister(register_index_first, register_index_second) => {
                self.add_register(register_index_first, register_index_second)
            }
            Operation::SubRegister(register_index_first, register_index_second) => {
                self.sub_register(register_index_first, register_index_second)
            }
            Operation::ShiftRight(register_index_first, register_index_second) => {
                self.shift_right(register_index_first, register_index_second)
            }
            Operation::ShiftLeft(register_index_first, register_index_second) => {
                self.shift_left(register_index_first, register_index_second)
            }
            Operation::SubRegisterReversed(register_index_first, register_index_second) => {
                self.sub_register_reversed(register_index_first, register_index_second)
            }
            Operation::IncrementIndexRegister(register_index) => {
                self.increment_index_register(register_index)
            }
            Operation::ToDecimal(register_index) => self.to_decimal(register_index),
            Operation::WriteMemory(register_index) => self.write_memory(register_index),
            Operation::ReadMemory(register_index) => self.read_memory(register_index),
            Operation::Return => self.return_()?,
            Operation::Call(address) => self.call(address)?,
            operation => todo!("{:?}", operation),
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

    fn add_value(&mut self, register_index: Nibble, word: u8) {
        self.registers.set(
            register_index,
            self.registers.get(register_index).overflowing_add(word).0,
        )
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

    fn skip_if_equal(&mut self, register_index: Nibble, word: u8) {
        if self.registers.get(register_index) == word {
            self.memory.increment_instruction_pointer();
        }
    }

    fn skip_if_not_equal(&mut self, register_index: Nibble, word: u8) {
        if self.registers.get(register_index) != word {
            self.memory.increment_instruction_pointer();
        }
    }

    fn skip_if_registers_equal(
        &mut self,
        register_index_first: Nibble,
        register_index_second: Nibble,
    ) {
        if self.registers.get(register_index_first) == self.registers.get(register_index_second) {
            self.memory.increment_instruction_pointer();
        }
    }

    fn skip_if_registers_not_equal(
        &mut self,
        register_index_first: Nibble,
        register_index_second: Nibble,
    ) {
        if self.registers.get(register_index_first) != self.registers.get(register_index_second) {
            self.memory.increment_instruction_pointer();
        }
    }

    fn set_to_register(&mut self, register_index_first: Nibble, register_index_second: Nibble) {
        self.registers.set(
            register_index_first,
            self.registers.get(register_index_second),
        )
    }

    fn or(&mut self, register_index_first: Nibble, register_index_second: Nibble) {
        let word =
            self.registers.get(register_index_first) | self.registers.get(register_index_second);

        self.registers.set(register_index_first, word);
    }

    fn and(&mut self, register_index_first: Nibble, register_index_second: Nibble) {
        let word =
            self.registers.get(register_index_first) & self.registers.get(register_index_second);

        self.registers.set(register_index_first, word);
    }

    fn xor(&mut self, register_index_first: Nibble, register_index_second: Nibble) {
        let word =
            self.registers.get(register_index_first) ^ self.registers.get(register_index_second);

        self.registers.set(register_index_first, word);
    }

    fn add_register(&mut self, register_index_first: Nibble, register_index_second: Nibble) {
        let (word, is_overflown) = self
            .registers
            .get(register_index_first)
            .overflowing_add(self.registers.get(register_index_second));

        self.registers.set(register_index_first, word);
        self.set_register_f(is_overflown)
    }

    fn sub_register(&mut self, register_index_first: Nibble, register_index_second: Nibble) {
        let (word, is_overflown) = self
            .registers
            .get(register_index_first)
            .overflowing_sub(self.registers.get(register_index_second));

        self.registers.set(register_index_first, word);
        self.set_register_f(!is_overflown)
    }

    fn shift_right(&mut self, register_index_first: Nibble, register_index_second: Nibble) {
        let word = self.registers.get(register_index_second);

        let shifted_word = word >> 1;
        let is_shifted_out = (word & 0b1) != 0;

        self.registers.set(register_index_first, shifted_word);
        self.set_register_f(is_shifted_out);
    }

    fn shift_left(&mut self, register_index_first: Nibble, register_index_second: Nibble) {
        let word = self.registers.get(register_index_second);

        let shifted_word = word << 1;
        let is_shifted_out = (word & 0b10000000) != 0;

        self.registers.set(register_index_first, shifted_word);
        self.set_register_f(is_shifted_out);
    }

    fn sub_register_reversed(
        &mut self,
        register_index_first: Nibble,
        register_index_second: Nibble,
    ) {
        let (word, is_overflown) = self
            .registers
            .get(register_index_second)
            .overflowing_sub(self.registers.get(register_index_first));

        self.registers.set(register_index_first, word);
        self.set_register_f(!is_overflown)
    }

    fn increment_index_register(&mut self, register_index: Nibble) {
        self.index_register += self.registers.get(register_index) as Offset
    }

    fn to_decimal(&mut self, register_index: Nibble) {
        let word = self.registers.get(register_index);

        let hundreds = word / 100;
        let tens = (word / 10) % 10;
        let units = word % 10;

        let base_index = self.index_register.as_usize();

        self.memory.locations[base_index] = hundreds;
        self.memory.locations[base_index + 1] = tens;
        self.memory.locations[base_index + 2] = units;
    }

    fn write_memory(&mut self, register_index: Nibble) {
        for i in 0..=register_index.as_u8() {
            self.memory.locations[self.index_register.as_usize() + usize::from(i)] =
                self.registers.get(Nibble(i))
        }

        self.index_register += register_index.as_offset() + 1;
    }

    fn read_memory(&mut self, register_index: Nibble) {
        for i in 0..=register_index.as_u8() {
            self.registers.set(
                Nibble(i),
                self.memory.locations[self.index_register.as_usize() + usize::from(i)],
            );
        }

        self.index_register += register_index.as_offset() + 1;
    }

    fn return_(&mut self) -> Result<()> {
        self.memory.instruction_pointer = self.stack.pop()?;

        Ok(())
    }

    fn call(&mut self, address: Address) -> Result<()> {
        self.stack.push(self.memory.instruction_pointer)?;
        self.memory.instruction_pointer = address;

        Ok(())
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
        self.increment_instruction_pointer();

        OpCode::from_bytes(self.locations[ipa], self.locations[ipa + 1])
    }

    fn get_slice(&self, start: Address, size: usize) -> &[u8] {
        &self.locations[start.as_usize()..start.as_usize() + size]
    }

    fn increment_instruction_pointer(&mut self) {
        self.instruction_pointer += 2
    }
}

struct Stack {
    stack: [Address; STACK_SIZE],
    pointer: usize,
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Stack {
    fn new() -> Self {
        Self {
            stack: [Address::new(0); STACK_SIZE],
            pointer: 0,
        }
    }

    fn push(&mut self, value: Address) -> Result<()> {
        if self.pointer + 1 == STACK_SIZE {
            return Err(Error::StackOverflow);
        }

        self.pointer += 1;
        self.stack[self.pointer] = value;

        Ok(())
    }

    fn pop(&mut self) -> Result<Address> {
        if self.pointer == 0 {
            return Err(Error::StackUnderflow);
        }

        self.pointer -= 1;
        let value = self.stack[self.pointer + 1];

        Ok(value)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy)]
pub enum Operation {
    ClearScreen,                                           // 00E0
    Return,                                                // 00EE
    Jump(Address),                                         // 1nnn
    Call(Address),                                         // 2nnn
    SkipIfEqual(RegisterIndex, Word),                      // 3xnn
    SkipIfNotEqual(RegisterIndex, Word),                   // 4xnn
    SkipIfRegistersEqual(RegisterIndex, RegisterIndex),    // 5xy0
    SetRegister(RegisterIndex, Word),                      // 6xnn
    AddValue(RegisterIndex, Word),                         // 7xnn
    SetToRegister(RegisterIndex, RegisterIndex),           // 8xy0
    Or(RegisterIndex, RegisterIndex),                      // 8xy1
    And(RegisterIndex, RegisterIndex),                     // 8xy2
    Xor(RegisterIndex, RegisterIndex),                     // 8xy3
    AddRegister(RegisterIndex, RegisterIndex),             // 8xy4
    SubRegister(RegisterIndex, RegisterIndex),             // 8xy5
    ShiftRight(RegisterIndex, RegisterIndex),              // 8xy6
    SubRegisterReversed(RegisterIndex, RegisterIndex),     // 8xy7
    ShiftLeft(RegisterIndex, RegisterIndex),               // 8xyE
    SkipIfRegistersNotEqual(RegisterIndex, RegisterIndex), // 9xy0
    SetIndexRegister(Address),                             // Annn
    JumpV0(Address),
    SetToRandom(RegisterIndex, Word),
    Draw(RegisterIndex, RegisterIndex, Nibble), // Dxyn
    SkipIfKeyDown(RegisterIndex),
    SkipIfKeyUp(RegisterIndex),
    GetDelayTimer(RegisterIndex),
    WaitForKey(RegisterIndex),
    SetDelayTimer(RegisterIndex),
    SetSoundTimer(RegisterIndex),
    IncrementIndexRegister(RegisterIndex), // Fx1E
    SetIndexRegisterToSprite(Nibble),
    ToDecimal(RegisterIndex), // Fx33
    WriteMemory(Nibble),      // Fx55
    ReadMemory(Nibble),       // Fx65
}

impl TryFrom<OpCode> for Operation {
    type Error = Error;

    fn try_from(op_code: OpCode) -> core::result::Result<Self, Error> {
        let unknown_op_code_error = Err(Error::UnknownOpCode(op_code));

        let operation = match op_code.as_u16() {
            0x00e0 => Self::ClearScreen,
            0x00ee => Self::Return,
            _ => match op_code.extract_nibble(0).as_u8() {
                0x1 => Self::Jump(op_code.extract_address()),
                0x2 => Self::Call(op_code.extract_address()),
                0x3 => Self::SkipIfEqual(op_code.extract_nibble(1), op_code.extract_word(1)),
                0x4 => Self::SkipIfNotEqual(op_code.extract_nibble(1), op_code.extract_word(1)),
                0x5 => {
                    Self::SkipIfRegistersEqual(op_code.extract_nibble(1), op_code.extract_nibble(2))
                }
                0x6 => Self::SetRegister(op_code.extract_nibble(1), op_code.extract_word(1)),
                0x7 => Self::AddValue(op_code.extract_nibble(1), op_code.extract_word(1)),
                0x8 => match op_code.extract_nibble(3).as_u8() {
                    0x0 => {
                        Self::SetToRegister(op_code.extract_nibble(1), op_code.extract_nibble(2))
                    }
                    0x1 => Self::Or(op_code.extract_nibble(1), op_code.extract_nibble(2)),
                    0x2 => Self::And(op_code.extract_nibble(1), op_code.extract_nibble(2)),
                    0x3 => Self::Xor(op_code.extract_nibble(1), op_code.extract_nibble(2)),
                    0x4 => Self::AddRegister(op_code.extract_nibble(1), op_code.extract_nibble(2)),
                    0x5 => Self::SubRegister(op_code.extract_nibble(1), op_code.extract_nibble(2)),
                    0x6 => Self::ShiftRight(op_code.extract_nibble(1), op_code.extract_nibble(2)),
                    0x7 => Self::SubRegisterReversed(
                        op_code.extract_nibble(1),
                        op_code.extract_nibble(2),
                    ),
                    0xe => Self::ShiftLeft(op_code.extract_nibble(1), op_code.extract_nibble(2)),
                    _ => return unknown_op_code_error,
                },
                0x9 => Self::SkipIfRegistersNotEqual(
                    op_code.extract_nibble(1),
                    op_code.extract_nibble(2),
                ),
                0xa => Self::SetIndexRegister(op_code.extract_address()),
                0xd => Self::Draw(
                    op_code.extract_nibble(1),
                    op_code.extract_nibble(2),
                    op_code.extract_nibble(3),
                ),
                0xf => match op_code.extract_word(1) {
                    0x1e => Self::IncrementIndexRegister(op_code.extract_nibble(1)),
                    0x33 => Self::ToDecimal(op_code.extract_nibble(1)),
                    0x55 => Self::WriteMemory(op_code.extract_nibble(1)),
                    0x65 => Self::ReadMemory(op_code.extract_nibble(1)),
                    _ => return unknown_op_code_error,
                },
                _ => return unknown_op_code_error,
            },
        };

        Ok(operation)
    }
}

////////////////////////////////////////////////////////////////////////////////
