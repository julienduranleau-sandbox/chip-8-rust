// http://devernay.free.fr/hacks/chip8/C8TECH10.HTM#2.1

use rand::prelude::*;

fn main() {
    let instructions = vec![0x54, 0x62];
    run_instructions(&instructions);
}

fn run_instructions(instructions: &Vec<u8>) {
    // 0x200 to 0xFFF : Chip-8 program / data
    // 0x000 to 0x1FF : Interpreter (do not use)
    let mut memory: [u8; 4096] = [0; 4096];

    // V0 to VF
    let mut registers: [u8; 16] = [0; 16];

    // Store memory addresses
    // Only 12 first lower bits are used
    let mut register_i: u16 = 0;

    // Decrement at 60hz
    let mut timer_sound: u8 = 0; // ST
    let mut timer_delay: u8 = 0; // DT

    // Currently executing address
    let mut pc: u16 = 0;

    // Point to the topmost level of the stack
    let mut sp: u16 = 0;

    // Store the address that the interpreter shoud return to when finished with a subroutine.
    // Chip-8 allows for up to 16 levels of nested subroutines.
    let mut stack: [u16; 16] = [0; 16];

    // Execution loop
    loop {
        let opcode_byte1 = instructions[pc as usize];
        let opcode_byte2 = instructions[(pc + 1) as usize];
        let opcode: u16 = ((opcode_byte1 as u16) << 8) | (opcode_byte2 as u16);
        println!("Opcode at {}: {:#018b} ({:#x})", pc, opcode, opcode);
        pc += 2;

        println!("{:#x}", (opcode & 0x000F) as u8);

        let nnn: u16 = opcode & 0x0FFF;
        let n: u8 = (opcode & 0x000F) as u8;
        let x: u8 = opcode_byte1 & 0x0f;
        let y: u8 = opcode_byte2 >> 4;
        let kk: u8 = (opcode & 0x00FF) as u8;

        // println!("x is {:#x}, y is {:#x}", x, y);

        match opcode & 0xF000 {
            0x0000 => {
                match opcode {
                    // 00E0 - CLS
                    0x00E0 => {
                        // Clear the display
                        // TODO
                    }
                    // 00EE - RET
                    0x00EE => {
                        // Return from a subroutine
                        pc = stack[sp as usize];
                        sp -= 1;
                    }
                    // 0nnn - SYS addr (ignored)
                    _ => {
                        //Jump to a machine code routine at nnn.
                        // This instruction is only used on the old computers on which Chip-8 was originally implemented. It is ignored by modern interpreters.
                    }
                }
            }

            // 1nnn - JP addr
            0x1000 => {
                // Jump to location nnn
                pc = nnn;
            }
            // 2nnn - CALL addr
            0x2000 => {
                // Call subroutine at nnn
                sp += 1;
                stack[sp as usize] = pc;
                pc = nnn;
            }
            // 3xkk - SE Vx, byte
            0x3000 => {
                // Skip next instruction if Vx = kk
                if registers[x as usize] == kk {
                    pc += 2;
                }
            }
            // 4xkk - SNE Vx, byte
            0x4000 => {
                // Skip next instruction if Vx != kk
                if registers[x as usize] != kk {
                    pc += 2;
                }
            }
            // 5xy0 - SE Vx, Vy
            0x5000 => {
                // Skip next instruction if Vx = Vy
                if registers[x as usize] == registers[y as usize] {
                    pc += 2;
                }
            }
            // 6xkk - LD Vx, byte
            0x6000 => {
                // Set Vx = kk
                registers[x as usize] = kk;
            }
            // 7xkk - ADD Vx, byte
            0x7000 => {
                // Set Vx = Vx + kk
                registers[x as usize] += kk;
            }
            0x8000 => {
                match opcode & 0x000F {
                    // 8xy0 - LD Vx, Vy
                    0x0 => {
                        // Set Vx = Vy
                        registers[x as usize] = registers[y as usize];
                    }
                    // 8xy1 - OR Vx, Vy
                    0x1 => {
                        // Set Vx = Vx OR Vy
                        registers[x as usize] |= registers[y as usize];
                    }
                    // 8xy2 - AND Vx, Vy
                    0x2 => {
                        // Set Vx = Vx AND Vy
                        registers[x as usize] &= registers[y as usize];
                    }
                    // 8xy3 - XOR Vx, Vy
                    0x3 => {
                        // Set Vx = Vx XOR Vy
                        registers[x as usize] ^= registers[y as usize];
                    }
                    // 8xy4 - ADD Vx, Vy
                    0x4 => {
                        // Set Vx = Vx + Vy, set VF = carry
                        let result =
                            (registers[x as usize] as u16) + (registers[y as usize] as u16);
                        registers[x as usize] = (result & 0x00FF) as u8;
                        registers[0xF] = if result > 0xFF { 1 } else { 0 }
                    }
                    // 8xy5 - SUB Vx, Vy
                    0x5 => {
                        // Set Vx = Vx - Vy, set VF = NOT borrow
                        registers[0xF] = if registers[x as usize] > registers[y as usize] {
                            1
                        } else {
                            0
                        };
                        registers[x as usize] -= registers[y as usize];
                    }
                    // 8xy6 - SHR Vx {, Vy}
                    0x6 => {
                        // Set Vx = Vx SHR 1
                        registers[0xF] = registers[x as usize] & 0b00000001;
                        registers[x as usize] /= 2;
                    }
                    // 8xy7 - SUBN Vx, Vy
                    0x7 => {
                        // Set Vx = Vy - Vx, set VF = NOT borrow
                        registers[0xF] = if registers[y as usize] > registers[x as usize] {
                            1
                        } else {
                            0
                        };
                        registers[x as usize] = registers[y as usize] - registers[x as usize];
                    }
                    // 8xyE - SHL Vx {, Vy}
                    0xE => {
                        // Set Vx = Vx SHL 1
                        registers[0xF] = if registers[x as usize] & 0b10000000 == 0b10000000 {
                            1
                        } else {
                            0
                        };
                        registers[x as usize] /= 2;
                    }
                    _ => {}
                }
            }
            // 9xy0 - SNE Vx, Vy
            0x9000 => {
                // Skip next instruction if Vx != Vy
                if registers[x as usize] != registers[y as usize] {
                    pc += 2;
                }
            }
            // Annn - LD I, addr
            0xA000 => {
                // Set I = nnn
                register_i = nnn;
            }
            // Bnnn - JP V0, addr
            0xB000 => {
                // Jump to location nnn + V0
                pc = nnn + (registers[0] as u16);
            }
            // Cxkk - RND Vx, byte
            0xC000 => {
                // Set Vx = random byte AND kk
                registers[x as usize] = rand::random::<u8>() & kk;
            }
            // Dxyn - DRW Vx, Vy, nibble
            0xD000 => {
                // Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
                /*
                The interpreter reads n bytes from memory,
                starting at the address stored in I.
                These bytes are then displayed as sprites on screen
                at coordinates (Vx, Vy). Sprites are XORed onto
                the existing screen. If this causes any pixels
                to be erased, VF is set to 1, otherwise it is
                set to 0. If the sprite is positioned so part of it
                is outside the coordinates of the display, it wraps
                around to the opposite side of the screen.
                */
                // TODO
            }
            0xE000 => {
                match opcode & 0x00FF {
                    // Ex9E - SKP Vx
                    0x9E => {
                        // Skip next instruction if key with the value of Vx is pressed
                        // TODO
                        /*
                        if pressed {
                            pc += 2;
                        }
                        */
                    }
                    // ExA1 - SKNP Vx
                    0xA1 => {
                        // Skip next instruction if key with the value of Vx is not pressed
                        // TODO
                        /*
                        if !pressed {
                            pc += 2;
                        }
                        */
                    }
                    _ => {}
                }
            }
            0xF000 => {
                match opcode & 0x00FF {
                    // Fx07 - LD Vx, DT
                    0x07 => {
                        // Set Vx = delay timer value
                        registers[x as usize] = timer_delay;
                    }
                    // Fx0A - LD Vx, K
                    0x0A => {
                        // Wait for a key press, store the value of the key in Vx
                        // All execution stops until a key is pressed, then the value of that key is stored in Vx
                        // TODO
                    }
                    // Fx15 - LD DT, Vx
                    0x15 => {
                        // Set delay timer = Vx
                        timer_delay = registers[x as usize];
                    }
                    // Fx18 - LD ST, Vx
                    0x18 => {
                        // Set sound timer = Vx
                        timer_sound = registers[x as usize];
                    }
                    // Fx1E - ADD I, Vx
                    0x1E => {
                        // Set I = I + Vx
                        register_i += registers[x as usize] as u16;
                    }
                    // Fx29 - LD F, Vx
                    0x29 => {
                        // Set I = location of sprite for digit Vx
                        // The value of I is set to the location for the hexadecimal sprite corresponding to the value of Vx
                        // TODO
                    }
                    // Fx33 - LD B, Vx
                    0x33 => {
                        // Store BCD representation of Vx in memory locations I, I+1, and I+2
                        // The interpreter takes the decimal value of Vx, and places the hundreds digit in memory at location in I, the tens digit at location I+1, and the ones digit at location I+2.
                        memory[(register_i + 0) as usize] = registers[x as usize] / 100;
                        memory[(register_i + 1) as usize] = (registers[x as usize] % 100) / 10;
                        memory[(register_i + 2) as usize] = registers[x as usize] % 10;
                    }
                    // Fx55 - LD [I], Vx
                    0x55 => {
                        // Store registers V0 through Vx in memory starting at location I
                        // The interpreter copies the values of registers V0 through Vx into memory, starting at the address in I
                        // I itself is left unmodified
                        for i in 0..registers.len() {
                            memory[(register_i as usize) + i] = registers[i];
                        }
                    }
                    // Fx65 - LD Vx, [I]
                    0x65 => {
                        // Read registers V0 through Vx from memory starting at location I
                        // The interpreter reads values from memory starting at location I into registers V0 through Vx
                        // I itself is left unmodified
                        for i in 0..registers.len() {
                            registers[i] = memory[(register_i as usize) + i];
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        if (pc as usize) >= instructions.len() {
            break;
        }
    }
}
