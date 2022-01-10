#![allow(non_snake_case)]

use crate::io::*;

#[rustfmt::skip]
const FONTSET: [u8; 80] =
[
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

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const GAME_ROM_OFFSET: usize = 0x200;

pub struct Chip8<R>
where
    R: Random,
{
    memory: [u8; 4096],
    // 16 general purpose registers
    V: [u8; 16],
    // Pointer register
    I: u16,
    // Program counter
    PC: u16,

    // Special registers, when non-zero they decrement at a rate of 60Hz
    delay: u8,
    sound: u8,

    // Stack pointer and the 16 value stack
    SP: u8,
    stack: [u16; 16],

    // generic IO structs
    rand: R,

    gfx: [u8; SCREEN_WIDTH * SCREEN_HEIGHT],
    keyboard: [bool; 16],
}

impl<R> Chip8<R>
where
    R: Random,
{
    pub fn new(game: &[u8], rand: R) -> Self {
        let mut memory = [0; 4096];
        memory.copy_from_slice(&FONTSET);
        memory[GAME_ROM_OFFSET..].copy_from_slice(game);

        Chip8 {
            memory,
            V: [0; 16],
            I: 0,
            PC: 0,
            delay: 0,
            sound: 0,
            SP: 0,
            stack: [0; 16],
            rand,
            gfx: [0; SCREEN_WIDTH * SCREEN_HEIGHT],
            keyboard: [false; 16],
        }
    }

    pub fn execute_instruction(&mut self) {
        // instructions are 16bit MSB
        let instruction: u16 = ((self.memory[self.PC as usize] as u16) << 8)
            + self.memory[(self.PC as usize) + 1] as u16;

        // split into nibbles as the opcodes are based on nibbles
        let opcode = (
            (instruction & 0xF000) >> 12,
            (instruction & 0x0F00) >> 8,
            (instruction & 0x00F0) >> 4,
            instruction & 0x000F,
        );

        self.PC += 2;

        match opcode {
            // CLS
            (0, 0, 0xE, 0) => self.gfx.iter_mut().for_each(|m| *m = 0),
            // RET
            (0, 0, 0xE, 0xE) => self.PC = self.pop_stack() as u16,
            // JMP nnn
            (1, _, _, _) => self.PC = instruction & 0x0FFF,
            // CALL nnn
            (2, _, _, _) => {
                self.push_stack(self.PC);
                self.PC = instruction & 0x0FFF;
            }
            // SE Vx, byte
            (3, x, _, _) => {
                if self.V[x as usize] == (instruction & 0x00FF) as u8 {
                    self.PC += 2;
                }
            }
            // SNE Vx, byte
            (4, x, _, _) => {
                if self.V[x as usize] != (instruction & 0x00FF) as u8 {
                    self.PC += 2;
                }
            }
            // SE Vx, Vy
            (5, x, y, 0) => {
                if self.V[x as usize] == self.V[y as usize] {
                    self.PC += 2;
                }
            }
            // LD Vx, byte
            (6, x, _, _) => self.V[x as usize] = (instruction & 0x00FF) as u8,
            // ADD Vx, byte
            (7, x, _, _) => {
                self.V[x as usize] = self.V[x as usize].wrapping_add((instruction & 0x00FF) as u8)
            }
            // LD Vx, Vy
            (8, x, y, 0) => self.V[x as usize] = self.V[y as usize],
            // OR Vx, Vy
            (8, x, y, 1) => self.V[x as usize] |= self.V[y as usize],
            // AND Vx, Vy
            (8, x, y, 2) => self.V[x as usize] &= self.V[y as usize],
            // XOR Vx, Vy
            (8, x, y, 3) => self.V[x as usize] ^= self.V[y as usize],
            // ADD Vx, Vy
            (8, x, y, 4) => {
                let (res, carry) = self.V[x as usize].overflowing_add(self.V[y as usize]);
                self.V[x as usize] = res;
                self.V[0xF] = carry as u8;
            }
            // SUB Vx, Vy
            (8, x, y, 5) => {
                let (res, carry) = self.V[x as usize].overflowing_sub(self.V[y as usize]);
                self.V[x as usize] = res;
                self.V[0xF] = carry as u8;
            }
            // SHR Vx, Vy
            (8, x, y, 6) => {
                self.V[0xF] = if self.V[y as usize] & 1 != 0 { 1 } else { 0 };
                self.V[x as usize] = self.V[y as usize] >> 1;
            }
            // SUBN Vx, Vy
            (8, x, y, 7) => {
                let (res, carry) = self.V[y as usize].overflowing_sub(self.V[x as usize]);
                self.V[x as usize] = res;
                self.V[0xF] = carry as u8;
            }
            // SHL Vx, Vy
            (8, x, y, 8) => {
                self.V[0xF] = if self.V[y as usize] & 0x80 != 0 { 1 } else { 0 };
                self.V[x as usize] = self.V[y as usize] << 1;
            }
            // SNE Vx, Vy
            (9, x, y, 0) => {
                if self.V[x as usize] != self.V[y as usize] {
                    self.PC += 2;
                }
            }
            // LD I, addr
            (0xA, _, _, _) => self.I = instruction & 0x0FFF,
            // JP V0, addr
            (0xB, _, _, _) => self.PC = (instruction & 0x0FFF) + self.V[0] as u16,
            // RND Vx, byte
            (0xC, x, _, _) => {
                self.V[x as usize] = (instruction & 0x00FF) as u8 & self.rand.randint()
            }
            // DRW Vx, Vy, nibble
            (0xD, x, y, n) => {
                // Clear collision
                self.V[0xF] = 0;

                for yl in 0..n {
                    let pixels = self.memory[(self.I + yl) as usize];
                    for xl in 0..8 {
                        if pixels & (0x80 >> xl) != 0 {
                            // Collision detection
                            if self.gfx[(x + xl + ((y + yl) * 64)) as usize] == 1 {
                                self.V[0xF] = 1;
                            }
                            self.gfx[(x + xl + ((y + yl) * 64)) as usize] ^= 1
                        }
                    }
                }
            }
            // SKP Vx
            (0xE, x, 9, 0xE) => {
                if self.key_pressed(self.V[x as usize]) {
                    self.PC += 2;
                }
            }
            // SKNP Vx
            (0xE, x, 0xA, 1) => {
                if !self.key_pressed(self.V[x as usize]) {
                    self.PC += 2;
                }
            }
            // LD Vx, DT
            (0xF, x, 0, 7) => {
                self.V[x as usize] = self.delay;
            }
            // LD Vx, K
            (0xF, x, 0, 0xA) => {
                let mut pressed = false;

                for i in 0..16 {
                    if self.key_pressed(i) {
                        self.V[x as usize] = i;
                        pressed = true;
                    }
                }

                if !pressed {
                    self.PC -= 2;
                }
            }
            // LD DT, Vx
            (0xF, x, 1, 5) => self.delay = self.V[x as usize],
            // LD ST, Vx
            (0xF, x, 1, 8) => self.sound = self.V[x as usize],
            // ADD I, Vx
            (0xF, x, 1, 0xE) => self.I = self.I.wrapping_add(self.V[x as usize] as u16),
            // LD F, Vx
            (0xF, x, 2, 9) => self.I = (self.V[x as usize] * 5) as u16,
            // LD B, Vx
            (0xF, x, 3, 3) => {
                let vx = self.V[x as usize];
                self.memory[self.I as usize] = vx / 100;
                self.memory[self.I as usize + 1] = (vx / 10) % 10;
                self.memory[self.I as usize + 2] = vx % 10;
            }
            // LD [I], Vx
            (0xF, x, 5, 5) => {
                for i in 0..(x as usize) {
                    self.memory[self.I as usize + i] = self.V[i];
                }
            }
            // LD Vx, [I]
            (0xF, x, 6, 5) => {
                for i in 0..(x as usize) {
                    self.V[i] = self.memory[self.I as usize + i];
                }
            }

            (_, _, _, _) => panic!("Invalid instruction!"),
        }
    }

    fn push_stack(&mut self, val: u16) {
        self.stack[self.SP as usize] = val;
        self.SP += 1;
    }

    fn pop_stack(&mut self) -> u16 {
        self.SP -= 1;
        self.stack[self.SP as usize]
    }

    fn key_pressed(&self, key: u8) -> bool {
        self.keyboard[key as usize]
    }

    fn set_key(&mut self, key: u8, state: bool) {
        self.keyboard[key as usize] = state;
    }
}
