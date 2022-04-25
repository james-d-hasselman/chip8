use crate::registers::Address;
use crate::registers::AddressRegister;
use crate::registers::ProgramCounter;
use std::io::Read;

#[derive(Debug, PartialEq)]
pub struct Memory {
    bytes: [u8; 4096],
}

impl Memory {
    pub fn new() -> Memory {
        let mut bytes = [0; 4096];
        // 0
        bytes[0..5].clone_from_slice(&[0xF0, 0x90, 0x90, 0x90, 0xF0]);
        // 1
        bytes[5..10].clone_from_slice(&[0x20, 0x60, 0x20, 0x20, 0x70]);
        // 2
        bytes[10..15].clone_from_slice(&[0xF0, 0x10, 0xF0, 0x80, 0xF0]);
        // 3
        bytes[15..20].clone_from_slice(&[0xF0, 0x10, 0xF0, 0x10, 0xF0]);
        // 4
        bytes[20..25].clone_from_slice(&[0x90, 0x90, 0xF0, 0x10, 0x10]);
        // 5
        bytes[25..30].clone_from_slice(&[0xF0, 0x80, 0xF0, 0x10, 0xF0]);
        // 6
        bytes[30..35].clone_from_slice(&[0xF0, 0x80, 0xF0, 0x90, 0xF0]);
        // 7
        bytes[35..40].clone_from_slice(&[0xF0, 0x10, 0x20, 0x40, 0x40]);
        // 8
        bytes[40..45].clone_from_slice(&[0xF0, 0x90, 0xF0, 0x90, 0xF0]);
        // 9
        bytes[45..50].clone_from_slice(&[0xF0, 0x90, 0xF0, 0x10, 0xF0]);
        // A
        bytes[50..55].clone_from_slice(&[0xF0, 0x90, 0xF0, 0x90, 0x90]);
        // B
        bytes[55..60].clone_from_slice(&[0xE0, 0x90, 0xE0, 0x90, 0xE0]);
        // C
        bytes[60..65].clone_from_slice(&[0xF0, 0x80, 0x80, 0x80, 0xF0]);
        // D
        bytes[65..70].clone_from_slice(&[0xE0, 0x90, 0x90, 0x90, 0xE0]);
        // E
        bytes[70..75].clone_from_slice(&[0xF0, 0x80, 0xF0, 0x80, 0xF0]);
        // F
        bytes[75..80].clone_from_slice(&[0xF0, 0x80, 0xF0, 0x80, 0x80]);

        Memory { bytes: bytes }
    }

    pub fn fetch(&self, program_counter: &ProgramCounter) -> &[u8; 2] {
        let address = usize::from(program_counter);
        self.bytes[address..address + 2]
            .try_into()
            .expect("Memory corrupted. crashing.")
    }

    pub fn load(&self, i: &AddressRegister, number_of_bytes: u16) -> &[u8] {
        &self.bytes[usize::from(*i)..(usize::from(*i) + number_of_bytes as usize)]
    }

    pub fn load_rom(&mut self, rom: &Vec<u8>) {
        let mut memory_address = 0x200;
        for byte in rom.bytes() {
            self.bytes[memory_address] = byte.unwrap();
            memory_address += 1;
        }
    }

    pub fn store(&mut self, i: &AddressRegister, bytes: &[u8]) {
        let start: usize = (*i).into();
        let end = start + bytes.len();
        self.bytes[start..end].clone_from_slice(bytes);
    }
}

type StackPointer = u8;

pub struct Stack {
    frames: [Address; 16],
    stack_pointer: StackPointer,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            frames: [Address::new(); 16],
            stack_pointer: 0,
        }
    }

    pub fn pop(&mut self) -> Option<Address> {
        if self.stack_pointer > 0 {
            let top = (self.stack_pointer - 1) as usize;
            let frame = self.frames[top];
            self.frames[self.stack_pointer as usize] = Address::from(0);
            self.stack_pointer -= 1;
            Some(frame)
        } else {
            None
        }
    }

    pub fn push(&mut self, address: &Address) {
        if self.stack_pointer + 1 >= 16 {
            panic!("Exceeded maximum stack frames, aborting.");
        } else {
            self.frames[self.stack_pointer as usize] = *address;
            self.stack_pointer += 1;
        }
    }

    #[allow(dead_code)]
    pub fn peek(&self) -> Option<Address> {
        if self.stack_pointer == 0 {
            None
        } else {
            let top = (self.stack_pointer - 1) as usize;
            let temp = self.frames[top];
            Some(temp)
        }
    }
}

#[test]
fn memory_load_rom() {
    let rom = vec![0xAA, 0xBB, 0xCC];
    let mut memory = Memory::new();
    memory.load_rom(&rom);
    let memory_cmp = Memory::new();
    assert_ne!(memory, memory_cmp);
}

#[test]
    fn memory_load() {
        let memory = Memory::new();
        let mut address_register = AddressRegister::new();
        address_register.set(Address::from(0));
        assert_eq!(
            memory.load(&address_register, 5),
            &[0xF0, 0x90, 0x90, 0x90, 0xF0]
        );
    }
    
    #[test]
    fn memory_store() {
        let data = [0xAA, 0xBB, 0xCC];
        let mut memory = Memory::new();
        let mut address_register = AddressRegister::new();
        address_register.set(Address::from(0x200));
        memory.store(&address_register, &data);
        assert_ne!(memory, Memory::new());
    }