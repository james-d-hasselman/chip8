use crate::audio::Buzzer;
use crate::graphics;
use crate::graphics::Display;
use crate::keyboard::Keyboard;
use crate::memory;
use crate::memory::Memory;
use crate::memory::Stack;
use crate::registers::Address;
use crate::registers::AddressRegister;
use crate::registers::DelayTimer;
use crate::registers::ProgramCounter;
use crate::registers::Register;
use crate::registers::SoundTimer;
use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

pub struct Interpreter {
    memory: Memory,
    program_counter: ProgramCounter,
    display_screen: Box<dyn Display>,
    stack: Stack,
    address_register: AddressRegister,
    registers: Vec<Register>,
    delay_timer: DelayTimer,
    sound_timer: SoundTimer,
    is_timer_running: Arc<AtomicBool>,
    timer: Option<JoinHandle<()>>,
    buzzer: Box<dyn Buzzer>,
    keyboard_device: Box<dyn Keyboard>,
}

impl Interpreter {
    pub fn new(
        display: Box<dyn Display>,
        buzzer: Box<dyn Buzzer>,
        keyboard_device: Box<dyn Keyboard>,
        rom: &Vec<u8>,
    ) -> Self {
        let mut memory = memory::Memory::new();
        memory.load_rom(rom);
        let delay_timer = DelayTimer::new();
        let sound_timer = SoundTimer::new();
        let thread_sound_timer = SoundTimer::clone(&sound_timer);
        let thread_delay_timer = DelayTimer::clone(&delay_timer);
        let is_timer_running = Arc::new(AtomicBool::new(true));
        let thread_is_timer_running = is_timer_running.clone();
        let timer = thread::spawn(move || while thread_is_timer_running.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(16));
            {
                let mut sound_timer_value = thread_sound_timer.lock().unwrap();
                if *sound_timer_value > 0 {
                    *sound_timer_value -= 1;
                }
            }
            {
                let mut delay_timer_value = thread_delay_timer.lock().unwrap();
                if *delay_timer_value > 0 {
                    *delay_timer_value -= 1;
                }
            }
        });
        Self {
            memory: memory,
            program_counter: ProgramCounter::new(),
            display_screen: display,
            stack: Stack::new(),
            address_register: AddressRegister::new(),
            registers: vec![Register::from(0); 16],
            delay_timer: delay_timer,
            sound_timer: sound_timer,
            is_timer_running: is_timer_running,
            timer: Some(timer),
            buzzer: buzzer,
            keyboard_device: keyboard_device,
        }
    }

    pub fn run_iteration(&mut self) {
        // fetch
        let instruction_code = self.memory.fetch(&self.program_counter);
        // increment
        self.program_counter.increment();
        // decode/execute
        let mut instruction_code = &instruction_code[..];
        let instruction_code = instruction_code.read_u16::<BigEndian>().unwrap();
        match instruction_code {
            0x00E0 => {
                // clear screen
                Interpreter::clear(&mut self.display_screen);
            }
            0x00EE => {
                Interpreter::return_subroutine(&mut self.program_counter, &mut self.stack);
            }
            code @ 0x0000..=0x0FFF
            | code @ 0x1000..=0x1FFF
            | code @ 0x2000..=0x2FFF
            | code @ 0xA000..=0xAFFF => {
                let address = Address::from(0x0FFF & code);
                match code >> 12 {
                    0x0 => {
                        // no-op
                    }
                    0x1 => {
                        Interpreter::jump_location_address(&address, &mut self.program_counter);
                    }
                    0x2 => {
                        Interpreter::call_address(
                            &mut self.stack,
                            &mut self.program_counter,
                            &address,
                        );
                    }
                    0xA => {
                        Interpreter::set_i_address(&mut self.address_register, &address);
                    }
                    0xB => {
                        Interpreter::jump_location_address_register(
                            &self.registers[0x0],
                            &address,
                            &mut self.program_counter,
                        );
                    }
                    _ => {
                        panic!("Invalic op code. crashing.")
                    }
                }
            }
            code @ 0x3000..=0x3FFF
            | code @ 0x4000..=0x4FFF
            | code @ 0x6000..=0x6FFF
            | code @ 0x7000..=0x7FFF
            | code @ 0xC000..=0xCFFF => {
                let register_number = ((0x0F00 & code) >> 8) as usize;
                let register = &mut self.registers[register_number];
                let byte = 0x00FF & code as u8;
                match code >> 12 {
                    0x3 => {
                        Interpreter::skip_if_equal_byte(register, byte, &mut self.program_counter);
                    }
                    0x4 => {
                        Interpreter::skip_if_not_equal_byte(
                            register,
                            byte,
                            &mut self.program_counter,
                        );
                    }
                    0x6 => {
                        Interpreter::load_byte(register, byte);
                    }
                    0x7 => {
                        Interpreter::add(register, byte);
                    }
                    0xC => {
                        Interpreter::random_and(register, byte);
                    }
                    _ => {
                        panic!("Invalid op-code {:#X}. crashing.", instruction_code);
                    }
                }
            }
            code @ 0x5000..=0x5FF0 | code @ 0x8000..=0x8FFE | code @ 0x9000..=0x9FF0 => {
                let vx = ((0x0F00 & code) >> 8) as usize;
                let vy = ((0x00F0 & code) >> 4) as usize;
                let register_y = self.registers[vy];
                let register_x = &mut self.registers[vx];
                match code >> 12 {
                    0x5 => {
                        Interpreter::skip_if_register_equal(
                            register_x,
                            &register_y,
                            &mut self.program_counter,
                        );
                    }
                    0x8 => match 0x000F & code {
                        0x0 => {
                            Interpreter::load_register(register_x, &register_y);
                        }
                        0x1 => {
                            Interpreter::bitwise_or_register(register_x, &register_y);
                        }
                        0x2 => {
                            Interpreter::bitwise_and_register(register_x, &register_y);
                        }
                        0x3 => {
                            Interpreter::bitwise_xor_register(register_x, &register_y);
                        }
                        0x4 => {
                            self.registers[0xF] = Interpreter::add_carry(register_x, &register_y);
                        }
                        0x5 => {
                            self.registers[0xF] =
                                Interpreter::subtract_register(register_x, &register_y);
                        }
                        0x6 => {
                            self.registers[0xF] = Interpreter::shift_right(register_x);
                        }
                        0x7 => {
                            self.registers[0xF] =
                                Interpreter::subtract_register_n(register_x, &register_y);
                        }
                        0xE => {
                            self.registers[0xF] = Interpreter::shift_left(register_x);
                        }
                        _ => {
                            panic!("Invalid op code {:#X}. crashing.", instruction_code);
                        }
                    },
                    0x9 => {
                        Interpreter::skip_if_register_not_equal(
                            &register_x,
                            &register_y,
                            &mut self.program_counter,
                        );
                    }
                    _ => {
                        panic!("Invalid op code. crashing.");
                    }
                }
            }
            code @ 0xD000..=0xDFFF => {
                let vx = ((0x0F00 & code) >> 8) as usize;
                let vy = ((0x00F0 & code) >> 4) as usize;
                let nibble = (0x000F & code) as u8;
                let register_x = self.registers[vx];
                let register_y = self.registers[vy];

                Interpreter::display(
                    &register_x,
                    &register_y,
                    nibble,
                    &mut self.registers[0xF],
                    &mut self.address_register,
                    &self.memory,
                    &mut self.display_screen,
                );
            }
            code @ 0xE09E..=0xEF9E
            | code @ 0xE0A1..=0xEFA1
            | code @ 0xF007..=0xFF07
            | code @ 0xF00A..=0xFF0A
            | code @ 0xF015..=0xFF15
            | code @ 0xF018..=0xFF18
            | code @ 0xF01E..=0xFF1E
            | code @ 0xF029..=0xFF29
            | code @ 0xF033..=0xFF33
            | code @ 0xF055..=0xFF55
            | code @ 0xF065..=0xFF65 => {
                let vx = ((0x0F00 & code) >> 8) as usize;
                let register_x = &mut self.registers[vx];
                match 0xF0FF & code {
                    0xE09E => {
                        Interpreter::skip_if_key(
                            register_x,
                            &mut self.program_counter,
                            &self.keyboard_device,
                        );
                    }
                    0xE0A1 => {
                        Interpreter::skip_if_not_key(
                            register_x,
                            &mut self.program_counter,
                            &self.keyboard_device,
                        );
                    }
                    0xF007 => {
                        Interpreter::load_delay_timer(register_x, &self.delay_timer);
                    }
                    0xF00A => {
                        Interpreter::load_on_key(
                            register_x,
                            &self.keyboard_device,
                            &mut self.program_counter,
                        );
                    }
                    0xF015 => {
                        Interpreter::set_delay_timer(&mut self.delay_timer, register_x);
                    }
                    0xF018 => {
                        Interpreter::set_sound_timer(&mut self.sound_timer, register_x, &self.buzzer);
                    }
                    0xF01E => {
                        Interpreter::add_address(&mut self.address_register, register_x);
                    }
                    0xF029 => {
                        Interpreter::set_i_sprite(&mut self.address_register, register_x);
                    }
                    0xF033 => {
                        Interpreter::load_bcd(register_x, &self.address_register, &mut self.memory);
                    }
                    0xF055 => {
                        Interpreter::load_range(
                            &self.registers[0..=vx],
                            &self.address_register,
                            &mut self.memory,
                        );
                    }
                    0xF065 => {
                        Interpreter::load_range_registers(
                            &mut self.registers[0..=vx],
                            &self.address_register,
                            &self.memory,
                        );
                    }
                    _ => {
                        panic!("Invalid op code {:#X}. crashing.", instruction_code);
                    }
                }
            }
            _ => {
                panic!("Invalid instruction {:#X}. crashing.", instruction_code);
            }
        }

        let sound_timer_value = self.sound_timer.lock().unwrap();
        if *sound_timer_value == 0 {
            self.buzzer.pause();
        }
    }

    // 0nnn - SYS addr
    // Jump to a machine code routine at nnn.
    #[allow(dead_code)]
    fn jump_address(self, _: &Address) {}

    // 00E0 - CLS
    // Clear the display
    fn clear(display: &mut Box<dyn Display>) {
        display.clear();
    }

    // 00EE - RET
    // Return from a subroutine
    fn return_subroutine(program_counter: &mut ProgramCounter, stack: &mut Stack) {
        program_counter.set(stack.pop().expect("Stack corrupted. crashing."));
    }

    // 1nnn - JP addr
    // Jump to location nnn.
    fn jump_location_address(address: &Address, program_counter: &mut ProgramCounter) {
        program_counter.set(*address);
    }

    // 2nnn - CALL addr
    // Call subroutine at nnn.
    fn call_address(stack: &mut Stack, program_counter: &mut ProgramCounter, address: &Address) {
        let program_counter_address: Address = program_counter.value;
        stack.push(&program_counter_address);
        program_counter.set(*address);
    }

    // 3xkk - SE Vx, byte
    // Skip next instruction if Vx = kk
    fn skip_if_equal_byte(vx: &Register, byte: u8, program_counter: &mut ProgramCounter) {
        if vx == &byte {
            program_counter.increment();
        }
    }

    // 4xkk - SNE Vx, byte
    // Skip next instruction if Vx != kk
    fn skip_if_not_equal_byte(vx: &Register, byte: u8, program_counter: &mut ProgramCounter) {
        if vx != &byte {
            program_counter.increment();
        }
    }

    // 5xy0 - SE Vx, Vy
    // Skip next instruction if Vx = Vy
    fn skip_if_register_equal(vx: &Register, vy: &Register, program_counter: &mut ProgramCounter) {
        if vx == vy {
            program_counter.increment();
        }
    }

    // 6xkk - LD Vx, byte
    // Set Vx = kk
    fn load_byte(vx: &mut Register, byte: u8) {
        //*vx = Register::from(byte);
        vx.set(byte);
    }

    // 7xkk - ADD Vx, byte
    // Set Vx = Vx + kk
    fn add(vx: &mut Register, byte: u8) {
        *vx += byte;
    }

    // 8xy0 - LD Vx, Vy
    // Set Vx = Vy
    fn load_register(vx: &mut Register, vy: &Register) {
        *vx = *vy;
    }

    // 8xy1 - OR Vx, Vy
    // Set Vx = Vx OR Vy
    fn bitwise_or_register(vx: &mut Register, vy: &Register) {
        *vx |= *vy;
    }

    // 8xy2 - AND Vx, Vy
    // Set Vx = Vx AND Vy
    fn bitwise_and_register(vx: &mut Register, vy: &Register) {
        *vx &= *vy;
    }

    // 8xy3 - XOR Vx, Vy
    // Set Vx = Vx XOR Vy
    fn bitwise_xor_register(vx: &mut Register, vy: &Register) {
        *vx ^= *vy;
    }

    // 8xy4 - ADD Vx, Vy
    // Set Vx = Vx + Vy, set VF = carry
    fn add_carry(vx: &mut Register, vy: &Register) -> Register {
        let temp: u16 = u16::from(*vx) + u16::from(*vy);
        let result = if temp > std::u8::MAX as u16 {
            Register::from(1)
        } else {
            Register::from(0)
        };
        *vx = Register::from((temp & (std::u8::MAX as u16)) as u8);
        result
    }

    // 8xy5 - SUB Vx, Vy
    // Set Vx = Vx - Vy, set VF = NOT borrow
    fn subtract_register(vx: &mut Register, vy: &Register) -> Register {
        let result = if *vx >= *vy {
            Register::from(1)
        } else {
            Register::from(0)
        };
        *vx -= *vy;
        result
    }

    // 8xy6 - SHR Vx {, Vy}
    // Set Vx = Vx SHR 1
    fn shift_right(vx: &mut Register) -> Register {
        let result = if (*vx & 0b00000001) == 1 {
            Register::from(1)
        } else {
            Register::from(0)
        };
        *vx >>= 1;
        result
    }

    // 8xy7 - SUBN Vx, Vy
    // Set Vx = Vy - Vx, set VF = NOT borrow.
    fn subtract_register_n(vx: &mut Register, vy: &Register) -> Register {
        let result = if *vy >= *vx {
            Register::from(1)
        } else {
            Register::from(0)
        };
        //*vx = *vy - *vx;
        vx.set((*vy - *vx).into());
        result
    }

    // 8xyE - SHL Vx {, Vy}
    // Set Vx = Vx SHL 1.
    fn shift_left(vx: &mut Register) -> Register {
        let result = if (*vx & 0b10000000) == 0b10000000 {
            Register::from(1)
        } else {
            Register::from(0)
        };
        *vx <<= 1;
        result
    }

    // 9xy0 - SNE Vx, Vy
    // Skip next instruction if Vx != Vy.
    fn skip_if_register_not_equal(
        vx: &Register,
        vy: &Register,
        program_counter: &mut ProgramCounter,
    ) {
        if *vx != *vy {
            program_counter.increment();
        }
    }

    // Annn - LD I, addr
    // Set I = nnn.
    fn set_i_address(i: &mut AddressRegister, address: &Address) {
        i.set(*address);
    }

    // Bnnn - JP V0, addr
    // Jump to location nnn + V0.
    fn jump_location_address_register(
        v0: &Register,
        address: &Address,
        program_counter: &mut ProgramCounter,
    ) {
        *program_counter = ProgramCounter {
            value: *address + *v0,
        };
    }

    // Cxkk - RND Vx, byte
    // Set Vx = random byte AND kk.
    fn random_and(vx: &mut Register, byte: u8) {
        let random_value: u8 = 27;
        *vx = Register::from(byte & random_value);
    }

    // Dxyn - DRW Vx, Vy, nibble
    // Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
    fn display(
        vx: &Register,
        vy: &Register,
        number_of_bytes: u8,
        vf: &mut Register,
        address: &mut AddressRegister,
        memory: &memory::Memory,
        display: &mut Box<dyn Display>,
    ) {
        let sprite = graphics::Sprite::from(memory.load(address, number_of_bytes as u16));
        vf.set(display.draw(u8::from(*vx), u8::from(*vy), &sprite));
        //display.show();
    }

    // Ex9E - SKP Vx
    // Skip next instruction if key with the value of Vx is pressed.
    fn skip_if_key(
        vx: &Register,
        program_counter: &mut ProgramCounter,
        keyboard: &Box<dyn Keyboard>,
    ) {
        if keyboard.is_key_down(u8::from(*vx)) {
            program_counter.increment();
        }
    }

    // ExA1 - SKNP Vx
    // Skip next instruction if key with the value of Vx is not pressed.
    fn skip_if_not_key(
        vx: &Register,
        program_counter: &mut ProgramCounter,
        keyboard: &Box<dyn Keyboard>,
    ) {
        if !keyboard.is_key_down(u8::from(*vx)) {
            program_counter.increment();
        }
    }

    // Fx07 - LD Vx, DT
    // Set Vx = delay timer value.
    fn load_delay_timer(vx: &mut Register, delay_timer: &DelayTimer) {
        *vx = Register::from(*delay_timer.lock().unwrap());
    }

    // Fx0A - LD Vx, K
    // Wait for a key press, store the value of the key in Vx.
    fn load_on_key(
        vx: &mut Register,
        keyboard: &Box<dyn Keyboard>,
        program_counter: &mut ProgramCounter,
    ) {
        match keyboard.get_pressed_key() {
            Some(k) => *vx = Register::from(k),
            _ => program_counter.decrement(),
        }
    }

    // Fx15 - LD DT, Vx
    // Set delay timer = Vx.
    fn set_delay_timer(delay_timer: &mut DelayTimer, vx: &Register) {
        //*delay_timer = DelayTimer::from(*vx);
        let mut delay_timer_value = delay_timer.lock().unwrap();
        *delay_timer_value = u8::from(*vx);
    }

    // Fx18 - LD ST, Vx
    // Set sound timer = Vx.
    fn set_sound_timer(sound_timer: &SoundTimer, vx: &Register, buzzer: &Box<dyn Buzzer>) {
        let mut sound_timer_value = sound_timer.lock().unwrap();
        *sound_timer_value = u8::from(*vx);
        buzzer.play();
    }
    // Fx1E - ADD I, Vx
    // Set I = I + Vx.
    fn add_address(i: &mut AddressRegister, vx: &Register) {
        *i += *vx;
    }

    // Fx29 - LD F, Vx
    // Set I = location of sprite for digit Vx.
    fn set_i_sprite(i: &mut AddressRegister, vx: &Register) {
        i.set(Address::from(0x000 + (5 * u16::from(*vx))));
    }

    // Fx33 - LD B, Vx
    // Store BCD representation of Vx in memory locations I, I+1, and I+2.
    fn load_bcd(vx: &Register, i: &AddressRegister, memory: &mut memory::Memory) {
        memory.store(
            i,
            &[
                (u8::from(*vx) / 100) % 10,
                (u8::from(*vx) / 10) % 10,
                u8::from(*vx) % 10,
            ],
        );
    }

    // Fx55 - LD [I], Vx
    // Store registers V0 through Vx in memory starting at location I.
    fn load_range(registers: &[Register], i: &AddressRegister, memory: &mut memory::Memory) {
        let mut bytes = Vec::new();
        for register in registers {
            bytes.push(u8::from(*register));
        }
        memory.store(i, &bytes[..]);
    }

    // Fx65 - LD Vx, [I]
    // Read registers V0 through Vx from memory starting at location I.
    fn load_range_registers(
        registers: &mut [Register],
        i: &AddressRegister,
        memory: &memory::Memory,
    ) {
        let bytes = memory.load(i, registers.len() as u16);
        for (index, byte) in bytes.iter().enumerate() {
            let x = &mut registers[index];
            *x = Register::from(*byte);
        }
    }
}

impl Drop for Interpreter {
    fn drop(&mut self) {
        self.is_timer_running.store(false, Ordering::Relaxed);
        self.timer.take().map(JoinHandle::join);
    }
}

/*#[cfg(test)]
mod tests {
    use crate::memory::Memory;
    use crate::memory::Stack;
    use crate::Address;
    use crate::AddressRegister;
    use crate::Keyboard;
    use crate::ProgramCounter;
    use crate::Register;
    use byteorder::BigEndian;
    use byteorder::ReadBytesExt;

    #[test]
    fn memory_fetch() {
        let mut program_counter = ProgramCounter::new();
        program_counter.set(Address::from(0x0000));
        let memory = Memory::new();
        let instruction_code = memory.fetch(&program_counter);
        let mut instruction_code = &instruction_code[..];
        let instruction_code = instruction_code.read_u16::<BigEndian>().unwrap();
        assert_eq!(instruction_code, 0xF090);
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
    // TODO improve testability
    /*#[test]
    fn memory_load_rom() {
        let rom = vec![0xAA, 0xBB, 0xCC];
        let mut memory = Memory::new();
        memory.load_rom(&rom);
        let mut memory_cmp = Memory::new();
        assert_ne!(memory, memory_cmp);
    }*/
    // TODO improve testability
    /*#[test]
    fn memory_store() {
        let data = [0xAA, 0xBB, 0xCC];
        let mut memory = Memory::new();
        let mut address_register = AddressRegister::new();
        address_register.0 = 0x200;
        memory.store(&address_register, &data);
        assert_ne!(memory, Memory::new());
    }*/
    #[test]
    fn stack_push() {
        let mut stack = Stack::new();
        match stack.peek() {
            None => assert!(true),
            _ => assert!(false),
        }
        stack.push(&Address::from(0x200));
        match stack.peek() {
            Some(value) => assert_eq!(value, Address::from(0x200)),
            _ => assert!(false),
        }
    }
    #[test]
    fn stack_pointer_pop() {
        let mut stack = Stack::new();
        stack.push(&Address::from(0x200));
        match stack.peek() {
            Some(value) => {
                assert_eq!(value, Address::from(0x200))
            }
            _ => assert!(false),
        }
        stack.pop();
        match stack.peek() {
            None => assert!(true),
            _ => assert!(false),
        }
    }
    // TODO make test friendly
    /*fn clear() {

    }*/
    #[test]
    fn test_return_subroutine() {
        let mut program_counter = ProgramCounter::new();
        assert_eq!(program_counter, 0x200);
        let mut stack = Stack::new();
        stack.push(&Address::from(0xAAAA));
        return_subroutine(&mut program_counter, &mut stack);
        assert_eq!(program_counter, 0xAAAA);
    }
    #[test]
    fn test_jump_location_address() {
        let mut program_counter = ProgramCounter::new();
        let address = Address::from(0xBBBB);
        assert_eq!(program_counter, 0x200);
        jump_location_address(&address, &mut program_counter);
        assert_eq!(program_counter, 0xBBBB);
    }
    #[test]
    fn test_call_address() {
        let mut stack = Stack::new();
        let mut program_counter = ProgramCounter::new();
        let address = Address::from(0xCCCC);
        assert_eq!(program_counter, 0x200);
        program_counter.set(Address::from(0xDDDD));
        match stack.peek() {
            None => assert!(true),
            _ => assert!(false),
        }
        call_address(&mut stack, &mut program_counter, &address);
        match stack.peek() {
            Some(value) => assert_eq!(value, Address::from(0xDDDD)),
            None => assert!(false),
        }
        assert_eq!(program_counter, Address::from(0xCCCC));
    }
    #[test]
    fn test_skip_if_equal_byte() {
        let vx = Register::from(0xAB);
        let byte = 0xAB;
        let mut program_counter = ProgramCounter::new();
        assert_eq!(program_counter, 0x200);
        skip_if_equal_byte(&vx, byte, &mut program_counter);
        assert_eq!(program_counter, 0x202);
    }
    #[test]
    fn test_skip_if_not_equal_byte() {
        let vx = Register::from(0xAB);
        let byte = 0xAA;
        let mut program_counter = ProgramCounter::new();
        assert_eq!(program_counter, 0x200);
        skip_if_not_equal_byte(&vx, byte, &mut program_counter);
        assert_eq!(program_counter, 0x202);
    }
    #[test]
    fn test_skip_if_register_equal() {
        let vx = Register::from(0xAA);
        let vy = Register::from(0xAA);
        let mut program_counter = ProgramCounter::new();
        assert_eq!(program_counter, 0x200);
        skip_if_register_equal(&vx, &vy, &mut program_counter);
        assert_eq!(program_counter, 0x202);
    }
    #[test]
    fn test_load_byte() {
        let mut vx = Register::from(0);
        let byte = 0xEE;
        load_byte(&mut vx, byte);
        assert_eq!(vx, byte);
    }
    #[test]
    fn test_add() {
        let mut vx = Register::from(0);
        let byte = 5;
        add(&mut vx, byte);
        assert_eq!(vx, 5);
    }
    #[test]
    fn test_load_register() {
        let mut vx = Register::from(0);
        let vy = Register::from(5);
        load_register(&mut vx, &vy);
        assert_eq!(vx, 5);
    }
    #[test]
    fn test_bitwise_or_register() {
        let mut vx = Register::from(0b11110000);
        let vy = Register::from(0b00001111);
        bitwise_or_register(&mut vx, &vy);
        assert_eq!(vx, 0b11111111);
    }
    #[test]
    fn test_bitwise_and_register() {
        let mut vx = Register::from(0b11111110);
        let vy = Register::from(0b10000001);
        bitwise_and_register(&mut vx, &vy);
        assert_eq!(vx, 0b10000000);
    }
    #[test]
    fn test_bitwise_xor_register() {
        let mut vx = Register::from(0b11110000);
        let vy = Register::from(0b00001111);
        bitwise_xor_register(&mut vx, &vy);
        assert_eq!(vx, 0b11111111);
    }
    #[test]
    fn test_add_carry() {
        let mut vx = Register::from(0xFF);
        let vy = Register::from(0x01);
        let mut vf = Register::from(0x00);
        vf = add_carry(&mut vx, &vy);
        assert_eq!(vf, 1);
    }
    #[test]
    fn test_subtract_register() {
        let mut vx = Register::from(10);
        let vy = Register::from(9);
        let mut vf = Register::from(0);
        vf = subtract_register(&mut vx, &vy);
        assert_eq!(vf, 1);
    }
    #[test]
    fn test_shift_right() {
        let mut vx = Register::from(4);
        shift_right(&mut vx);
        assert_eq!(vx, 2);
    }
    #[test]
    fn test_subtract_register_n() {
        let mut vx = Register::from(10);
        let vy = Register::from(9);
        let mut vf = Register::from(0);
        vf = subtract_register_n(&mut vx, &vy);
        assert_eq!(vf, 0);
    }
    #[test]
    fn test_shift_left() {
        let mut vx = Register::from(2);
        shift_left(&mut vx);
        assert_eq!(vx, 4);
    }
    #[test]
    fn test_skip_if_register_not_equal() {
        let vx = Register::from(0);
        let vy = Register::from(1);
        let mut program_counter = ProgramCounter::new();
        assert_eq!(program_counter, 0x200);
        skip_if_register_not_equal(&vx, &vy, &mut program_counter);
        assert_eq!(program_counter, 0x202);
    }
    #[test]
    fn test_set_i_address() {
        let mut i = AddressRegister::new();
        assert_eq!(i, 0);
        set_i_address(&mut i, &Address::from(0xBBBB));
        assert_eq!(i, 0xBBBB);
    }
    #[test]
    fn test_jump_location_address_register() {
        let v0 = Register::from(0xF1);
        let address = Address::from(0x400);
        let mut program_counter = ProgramCounter::new();
        jump_location_address_register(&v0, &address, &mut program_counter);
        assert_eq!(program_counter, 0x4F1);
    }
    #[test]
    fn test_random_and() {
        let mut vx = Register::from(0xCD);
        random_and(&mut vx, 0xA1);
        assert_ne!(vx, 0xCD);
    }

    // TODO make more test friendly
    /*fn display() {

    }*/

    // TODO figure out how to make this work again
    /*#[test]
    fn test_skip_if_key() {
        let vx = Register::from(15);
        let mut program_counter = ProgramCounter::new();
        let mut keyboard = Keyboard::new();
        keyboard.keys.insert(vx.0, true);
        skip_if_key(&vx, &mut program_counter, &keyboard);
        assert_eq!(program_counter.value.0, 0x202);
    }*/
    #[test]
    fn test_skip_if_not_key() {
        let vx = Register::from(15);
        let mut program_counter = ProgramCounter::new();
        let keyboard = Keyboard::new();
        skip_if_not_key(&vx, &mut program_counter, &keyboard);
        assert_eq!(program_counter, 0x202);
    }
    /*#[test]
    fn test_load_delay_timer() {
        let mut vx = Register::from(66);
        let delay_timer = DelayTimer::new();
        load_delay_timer(&mut vx, &delay_timer);
        assert_eq!(vx.0, delay_timer.0);
    }*/

    // TODO figure out how to make this work again
    /*#[test]
    fn test_load_on_key() {
        let mut vx = Register::from(0);
        let mut keyboard = Keyboard::new();
        let x = 7 as u8;
        keyboard.keys.insert(x, true);
        let mut program_counter = ProgramCounter::new();
        load_on_key(&mut vx, &keyboard, &mut program_counter);
        assert_eq!(program_counter.value.0, 0x200);
        assert_eq!(vx.0, 7);
    }*/
    /*#[test]
    fn test_set_delay_timer() {
        let mut delay_timer = DelayTimer::new();
        let vx = Register::from(6);
        set_delay_timer(&mut delay_timer, &vx);
        assert_eq!(delay_timer.0, 6);
    }*/

    // TODO make test friendly
    /*fn test_set_sound_timer() {
        set_sound_timer(&sound_timer, &vx, &buzzer)
    }*/
    #[test]
    fn test_add_address() {
        let mut i = AddressRegister::new();
        i.set(Address::from(0x0100));
        let vx = Register::from(0x20);
        add_address(&mut i, &vx);
        assert_eq!(i, 0x0120);
    }
    #[test]
    fn test_set_i_sprite() {
        let mut i = AddressRegister::new();
        let vx = Register::from(5);
        set_i_sprite(&mut i, &vx);
        assert_eq!(i, 25);
    }
    #[test]
    fn test_load_bcd() {
        let mut memory = Memory::new();
        let vx = Register::from(123);
        let i = AddressRegister::new();
        i.set(Address::from(0x200));
        load_bcd(&vx, &i, &mut memory);
        assert_eq!(memory.load(&i, 1)[0], 1);
        let i = AddressRegister::new();
        i.set(Address::from(0x201));
        assert_eq!(memory.load(&i, 1)[0], 2);
        let i = AddressRegister::new();
        i.set(Address::from(0x202));
        assert_eq!(memory.load(&i, 1)[0], 3);
    }
    #[test]
    fn test_load_range() {
        let mut registers = vec![Register::from(9); 16];
        let i = AddressRegister::new();
        i.set(Address::from(0x500));
        let mut memory = Memory::new();
        load_range(&registers[..], &i, &mut memory);
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x501));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x502));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x503));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x504));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x505));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x506));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x507));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x508));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x509));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x50A));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x50B));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x50C));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x50D));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x50E));
        assert_eq!(memory.load(&i, 1)[0], 9);
        let i = AddressRegister::new();
        i.set(Address::from(0x50F));
        assert_eq!(memory.load(&i, 1)[0], 9);
    }
    #[test]
    fn test_load_range_registers() {
        let mut registers = vec![Register::from(0); 16];
        let i = AddressRegister::new();
        let mut memory = Memory::new();
        load_range_registers(&mut registers[0..5], &i, &memory);
        assert_eq!(registers[0], 0xF0);
        assert_eq!(registers[1], 0x90);
        assert_eq!(registers[2], 0x90);
        assert_eq!(registers[3], 0x90);
        assert_eq!(registers[4], 0xF0);
    }
}*/
