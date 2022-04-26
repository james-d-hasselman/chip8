/* chip8 - A cross platform CHIP-8 interpreter.
 * Copyright (C) 2022  James D. Hasselman
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::cmp::Ordering;
use std::ops::AddAssign;
use std::ops::BitAnd;
use std::ops::BitAndAssign;
use std::ops::BitOrAssign;
use std::ops::BitXorAssign;
use std::ops::ShlAssign;
use std::ops::ShrAssign;
use std::ops::Sub;
use std::ops::SubAssign;
use std::sync::Arc;
use std::sync::LockResult;
use std::sync::Mutex;
use std::sync::MutexGuard;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Address(u16);

impl Address {
    pub fn new() -> Self {
        Address(0)
    }
}

impl From<u16> for Address {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<Address> for u16 {
    fn from(address: Address) -> u16 {
        address.0
    }
}

impl From<Address> for usize {
    fn from(address: Address) -> usize {
        address.0 as usize
    }
}

impl std::ops::Add<Register> for Address {
    type Output = Self;

    fn add(self, other: Register) -> Self {
        Self(self.0.wrapping_add(other.0 as u16))
    }
}

impl std::ops::AddAssign<u16> for Address {
    fn add_assign(&mut self, other: u16) {
        self.0 = self.0.wrapping_add(other);
    }
}

impl std::ops::SubAssign<u16> for Address {
    fn sub_assign(&mut self, other: u16) {
        self.0 = self.0.wrapping_sub(other);
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AddressRegister(u16);

impl AddressRegister {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn set(&mut self, value: Address) {
        self.0 = value.0;
    }
}

impl std::ops::AddAssign<Register> for AddressRegister {
    fn add_assign(&mut self, register: Register) {
        *self = Self(self.0.wrapping_add(register.0 as u16))
    }
}

impl From<AddressRegister> for usize {
    fn from(i: AddressRegister) -> usize {
        i.0 as usize
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Register(u8);

impl Register {
    pub fn set(&mut self, value: u8) {
        self.0 = value;
    }
}

impl From<DelayTimer> for Register {
    fn from(delay_timer: DelayTimer) -> Self {
        Self(*delay_timer.0.lock().unwrap())
    }
}

impl From<SoundTimer> for Register {
    fn from(sound_timer: SoundTimer) -> Self {
        Self(*sound_timer.0.lock().unwrap())
    }
}
impl PartialEq<u8> for Register {
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }

    fn ne(&self, other: &u8) -> bool {
        self.0 != *other
    }
}

impl PartialEq for Register {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn ne(&self, other: &Self) -> bool {
        self.0 != other.0
    }
}

impl PartialOrd for Register {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl BitOrAssign for Register {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl BitAnd<u8> for Register {
    type Output = u8;
    fn bitand(self, value: u8) -> u8 {
        self.0 & value
    }
}

impl BitAndAssign for Register {
    fn bitand_assign(&mut self, other: Self) {
        self.0 &= other.0;
    }
}

impl BitXorAssign for Register {
    fn bitxor_assign(&mut self, other: Self) {
        self.0 ^= other.0;
    }
}

impl From<u8> for Register {
    fn from(byte: u8) -> Self {
        Self(byte)
    }
}

impl From<Register> for u8 {
    fn from(register: Register) -> u8 {
        register.0
    }
}

impl From<Register> for u16 {
    fn from(register: Register) -> u16 {
        register.0 as u16
    }
}

impl AddAssign<u8> for Register {
    fn add_assign(&mut self, byte: u8) {
        self.0 = self.0.wrapping_add(byte);
    }
}

impl Sub for Register {
    type Output = Register;
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0.wrapping_sub(other.0))
    }
}

impl SubAssign for Register {
    fn sub_assign(&mut self, other: Self) {
        self.0 = self.0.wrapping_sub(other.0)
    }
}

impl ShlAssign<usize> for Register {
    fn shl_assign(&mut self, bits: usize) {
        self.0 = self.0 << bits;
    }
}

impl ShrAssign<usize> for Register {
    fn shr_assign(&mut self, bits: usize) {
        self.0 = self.0 >> bits;
    }
}

#[derive(Debug, PartialEq)]
pub struct ProgramCounter {
    pub value: Address,
}

impl ProgramCounter {
    pub fn new() -> Self {
        Self {
            value: Address::from(0x200),
        }
    }

    pub fn increment(&mut self) {
        self.value += 2;
    }

    pub fn decrement(&mut self) {
        self.value -= 2;
    }

    pub fn set(&mut self, address: Address) {
        self.value = address;
    }
}

impl From<&Address> for ProgramCounter {
    fn from(address: &Address) -> Self {
        Self { value: *address }
    }
}

impl From<&ProgramCounter> for usize {
    fn from(program_counter: &ProgramCounter) -> usize {
        program_counter.value.into()
    }
}

#[derive(Debug)]
pub struct DelayTimer(Arc<Mutex<u8>>);

impl DelayTimer {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(0)))
    }

    pub fn lock(&self) -> LockResult<MutexGuard<'_, u8>> {
        self.0.lock()
    }
}

impl Clone for DelayTimer {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl From<Register> for DelayTimer {
    fn from(register: Register) -> Self {
        Self(Arc::new(Mutex::new(register.into())))
    }
}

pub struct SoundTimer(Arc<Mutex<u8>>);

impl SoundTimer {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(0)))
    }

    pub fn lock(&self) -> LockResult<MutexGuard<'_, u8>> {
        self.0.lock()
    }
}

impl Clone for SoundTimer {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
#[cfg(test)]
mod tests {
    use crate::registers::Address;
    use crate::registers::AddressRegister;
    use crate::registers::ProgramCounter;
    use crate::registers::Register;

    #[test]
    fn program_counter_increments() {
        let mut program_counter = ProgramCounter::new();
        assert_eq!(program_counter.value.0, 0x200);
        program_counter.increment();
        assert_eq!(program_counter.value.0, 0x202);
    }

    #[test]
    fn program_counter_decrements() {
        let mut program_counter = ProgramCounter::new();
        program_counter.value.0 = 0x202;
        assert_eq!(program_counter.value.0, 0x202);
        program_counter.decrement();
        assert_eq!(program_counter.value.0, 0x200);
    }

    #[test]
    fn address_add_assign() {
        let mut address = Address::new();
        assert_eq!(address.0, 0x00);
        address += 2;
        assert_eq!(address.0, 0x02);
    }

    #[test]
    fn address_sub_assign() {
        let mut address = Address::from(0x200);
        address -= 0x002;
        assert_eq!(address.0, 0x1FE);
    }

    #[test]
    fn address_add() {
        let address = Address::from(0x200);
        assert_eq!((address + Register::from(0x02)).0, Address::from(0x202).0);
    }

    #[test]
    fn address_register_add_assign() {
        let mut address_register = AddressRegister::new();
        assert_eq!(address_register.0, 0);
        address_register += Register::from(0x02);
        assert_eq!(address_register.0, 2);
        let mut address_register_2 = AddressRegister::new();
        address_register_2.0 = 0xFFFF;
        address_register_2 += Register::from(1);
        assert_eq!(address_register_2.0, 0);
    }

    #[test]
    fn register_add_assign() {
        let mut register = Register::from(0);
        register += 2;
        assert_eq!(register.0, 2);
    }
    #[test]
    fn register_bitand_assign() {
        let mut register = Register::from(0b11110000);
        register &= Register::from(0b11101111);
        assert_eq!(register.0, 0b11100000);
    }
    #[test]
    fn register_bitor_assign() {
        let mut register = Register::from(0b10101010);
        register |= Register::from(0b01010101);
        assert_eq!(register.0, 0b11111111);
    }
    #[test]
    fn register_bitxor_assign() {
        let mut register = Register::from(0b11111111);
        register ^= Register::from(0b11111111);
        assert_eq!(register.0, 0b00000000);
    }
    #[test]
    fn register_eq_u8() {
        let register = Register::from(0xFE);
        assert_eq!(register.0 == 0xFE, true);
    }
    #[test]
    fn register_ne_u8() {
        let register = Register::from(0xFE);
        assert_eq!(register.0 != 0xFA, true);
    }
    #[test]
    fn register_eq_register() {
        assert_eq!(Register::from(0xFE) == Register::from(0xFE), true);
    }
    #[test]
    fn register_ne_register() {
        assert_eq!(Register::from(0xFA) != Register::from(0xFE), true);
    }
    #[test]
    fn register_ge_register() {
        assert_eq!(Register::from(0xFF) >= Register::from(0xFF), true);
        assert_eq!(Register::from(0xFF) >= Register::from(0xFE), true);
    }
    #[test]
    fn register_gt_register() {
        assert_eq!(Register::from(0xFF) > Register::from(0xFE), true);
    }
    #[test]
    fn register_le() {
        assert_eq!(Register::from(0xFF) <= Register::from(0xFF), true);
        assert_eq!(Register::from(0xFE) <= Register::from(0xFF), true);
    }
    #[test]
    fn register_lt() {
        assert_eq!(Register::from(0xFE) < Register::from(0xFF), true);
    }
    #[test]
    fn register_shift_left_assign() {
        let mut register = Register::from(4);
        register <<= 1;
        assert_eq!(register.0, 8);
    }
    #[test]
    fn register_shift_right() {
        let mut register = Register::from(4);
        register >>= 1;
        assert_eq!(register.0, 2);
    }
}
