// http://devernay.free.fr/hacks/chip8/C8TECH10.HTM#2.1
// https://en.wikipedia.org/wiki/CHIP-8#Opcode_table
// https://github.com/nannou-org/nannou
// https://www.freecodecamp.org/news/creating-your-very-own-chip-8-emulator/

mod assembler;

use nannou::prelude::*;

const WIDTH: u8 = 64;
const HEIGHT: u8 = 32;
const SCALE: u8 = 10;
const WINDOW_WIDTH: u32 = WIDTH as u32 * SCALE as u32;
const WINDOW_HEIGHT: u32 = HEIGHT as u32 * SCALE as u32;
const VOLUME: f32 = 0.02;
const WAVE_LENGTH: u32 = 440;

struct Chip8 {
    display: [u8; WIDTH as usize * HEIGHT as usize],

    // 0x200 to 0xFFF : Chip-8 program / data
    // 0x000 to 0x1FF : Interpreter (do not use)
    memory: [u8; 4096],

    /*
    1 2	3 C  =>  1 2 3 4
    4 5	6 D  =>  q w e r
    7 8	9 E  =>  a s d t
    A 0	B F  =>  z x c v
    */
    keys: [bool; 16],

    // V0 to VF
    registers: [u8; 16],

    // Store memory addresses
    // Only 12 first lower bits are used
    register_i: u16,

    // Decrement at 60hz
    timer_sound: u8, // ST
    timer_delay: u8, // DT

    // Currently executing address
    pc: u16,

    // Point to the topmost level of the stack
    sp: u16,

    // Store the address that the interpreter shoud return to when finished with a subroutine.
    // Chip-8 allows for up to 16 levels of nested subroutines.
    stack: [u16; 16],

    // Transfer clear request from cpu to update
    needs_clear: bool,

    // Request a cpu hold until a key is pressed. Value of key (0x0..0xF) is stored in register
    hold_for_key: Option<u8>,

    // Thread channel. Send true to play sound, false to stop it
    audio_control_channel: std::sync::mpsc::Sender<bool>,

    // State variable for sound
    audio_is_playing: bool,
}

fn main() {
    nannou::app(model).update(update).view(view).run();
}

fn model(app: &App) -> Chip8 {
    let _window = app
        .new_window()
        .title("Chip-8")
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .key_pressed(key_pressed)
        .key_released(key_released)
        .build()
        .unwrap();

    let mut memory = [0; 4096];

    let digit_sprites = get_digit_sprites();

    for i in 0..digit_sprites.len() {
        memory[0x0 + i] = digit_sprites[i];
    }

    // let instructions = load_rom_from_file("roms/games/Pong 2 (Pong hack) [David Winter, 1997].ch8");
    let instructions = assembler::assemble("assembly_programs/clock.cp8asm");

    println!("===================================");
    println!("Starting emulation with {} opcodes.", instructions.len());

    for i in 0..instructions.len() {
        memory[0x200 + i] = instructions[i];
    }

    let (tx, rx) = std::sync::mpsc::channel();

    let _audio_thread_handle = std::thread::spawn(move || {
        let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();
        sink.set_volume(VOLUME);
        sink.pause();
        let source = rodio::source::SineWave::new(WAVE_LENGTH);
        sink.append(source);

        while let Ok(should_play) = rx.recv() {
            if should_play {
                sink.play();
            } else {
                sink.pause();
            }
        }
    });

    Chip8 {
        display: [0; WIDTH as usize * HEIGHT as usize],
        memory,
        keys: [false; 16],
        registers: [0; 16],
        register_i: 0,
        timer_sound: 0,
        timer_delay: 0,
        pc: 0x200,
        sp: 0,
        stack: [0; 16],
        needs_clear: false,
        hold_for_key: None,
        audio_control_channel: tx,
        audio_is_playing: false,
    }
}

fn update(_app: &App, chip8: &mut Chip8, _update: Update) {
    if chip8.timer_delay > 0 {
        chip8.timer_delay -= 1;
    }
    if chip8.timer_sound > 0 {
        chip8.timer_sound -= 1;
    }

    if chip8.hold_for_key.is_none() {
        // 500hz / 60fps = ~8 instructions per frame
        for _i in 0..8 {
            if chip8.pc < (chip8.memory.len() as u16 - 2) {
                run_next_cpu_cycle(chip8);
            }

            if !chip8.audio_is_playing && chip8.timer_sound > 0 {
                chip8.audio_is_playing = true;
                chip8.audio_control_channel.send(true).unwrap();
            } else if chip8.audio_is_playing && chip8.timer_sound == 0 {
                chip8.audio_is_playing = false;
                chip8.audio_control_channel.send(false).unwrap();
            }

            if chip8.needs_clear {
                chip8.display = [0; WIDTH as usize * HEIGHT as usize];
                chip8.needs_clear = false;
            }
        }
    }
}

fn view(app: &App, chip8: &Chip8, frame: Frame) {
    frame.clear(BLACK);
    let draw = app.draw();

    for i in 0..chip8.display.len() {
        let px = chip8.display[i];
        if px == 1 {
            let display_x = i % 64;
            let display_y = i / 64;

            let window_x = -(WINDOW_WIDTH as f32) / 2.0
                + display_x as f32 * SCALE as f32
                + (SCALE as f32) / 2.0;
            let window_y = (WINDOW_HEIGHT as f32) / 2.0
                - (display_y as f32) * SCALE as f32
                - (SCALE as f32) / 2.0;

            draw.rect()
                .x_y(window_x, window_y)
                .w_h(SCALE as f32, SCALE as f32)
                .color(WHITE);
        }
    }

    draw.to_frame(app, &frame).unwrap();
}

fn run_next_cpu_cycle(chip8: &mut Chip8) {
    // println!("PC: {}", chip8.pc);
    let opcode_byte1 = chip8.memory[chip8.pc as usize];
    let opcode_byte2 = chip8.memory[(chip8.pc + 1) as usize];
    let opcode: u16 = ((opcode_byte1 as u16) << 8) | (opcode_byte2 as u16);
    // println!("Opcode at {}: {:#018b} ({:#x})", chip8.pc, opcode, opcode);
    chip8.pc += 2;

    let nnn: u16 = opcode & 0x0FFF;
    let n: u8 = (opcode & 0x000F) as u8;
    let x: u8 = opcode_byte1 & 0x0f;
    let y: u8 = opcode_byte2 >> 4;
    let kk: u8 = (opcode & 0x00FF) as u8;

    match opcode & 0xF000 {
        0x0000 => {
            match opcode {
                // 00E0 - CLS
                0x00E0 => {
                    // Clear the display
                    chip8.needs_clear = true;
                }
                // 00EE - RET
                0x00EE => {
                    // Return from a subroutine
                    chip8.pc = chip8.stack[chip8.sp as usize];
                    chip8.sp -= 1;
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
            chip8.pc = nnn;
        }
        // 2nnn - CALL addr
        0x2000 => {
            // Call subroutine at nnn
            chip8.sp += 1;
            chip8.stack[chip8.sp as usize] = chip8.pc;
            chip8.pc = nnn;
        }
        // 3xkk - SE Vx, byte
        0x3000 => {
            // Skip next instruction if Vx = kk
            if chip8.registers[x as usize] == kk {
                chip8.pc += 2;
            }
        }
        // 4xkk - SNE Vx, byte
        0x4000 => {
            // Skip next instruction if Vx != kk
            if chip8.registers[x as usize] != kk {
                chip8.pc += 2;
            }
        }
        // 5xy0 - SE Vx, Vy
        0x5000 => {
            // Skip next instruction if Vx = Vy
            if chip8.registers[x as usize] == chip8.registers[y as usize] {
                chip8.pc += 2;
            }
        }
        // 6xkk - LD Vx, byte
        0x6000 => {
            // Set Vx = kk
            chip8.registers[x as usize] = kk;
        }
        // 7xkk - ADD Vx, byte
        0x7000 => {
            // Set Vx = Vx + kk
            let result = chip8.registers[x as usize] as u16 + kk as u16;
            chip8.registers[x as usize] = (result & 0xFF) as u8
        }
        0x8000 => {
            match opcode & 0x000F {
                // 8xy0 - LD Vx, Vy
                0x0 => {
                    // Set Vx = Vy
                    chip8.registers[x as usize] = chip8.registers[y as usize];
                }
                // 8xy1 - OR Vx, Vy
                0x1 => {
                    // Set Vx = Vx OR Vy
                    chip8.registers[x as usize] |= chip8.registers[y as usize];
                }
                // 8xy2 - AND Vx, Vy
                0x2 => {
                    // Set Vx = Vx AND Vy
                    chip8.registers[x as usize] &= chip8.registers[y as usize];
                }
                // 8xy3 - XOR Vx, Vy
                0x3 => {
                    // Set Vx = Vx XOR Vy
                    chip8.registers[x as usize] ^= chip8.registers[y as usize];
                }
                // 8xy4 - ADD Vx, Vy
                0x4 => {
                    // Set Vx = Vx + Vy, set VF = carry
                    let result =
                        (chip8.registers[x as usize] as u16) + (chip8.registers[y as usize] as u16);
                    chip8.registers[x as usize] = (result & 0xFF) as u8;
                    chip8.registers[0xF] = if result > 0xFF { 1 } else { 0 }
                }
                // 8xy5 - SUB Vx, Vy
                0x5 => {
                    // Set Vx = Vx - Vy, set VF = NOT borrow
                    chip8.registers[0xF] =
                        if chip8.registers[x as usize] > chip8.registers[y as usize] {
                            1
                        } else {
                            0
                        };
                    chip8.registers[x as usize] =
                        match chip8.registers[x as usize].checked_sub(chip8.registers[y as usize]) {
                            Some(n) => n,
                            None => 0,
                        }
                }
                // 8xy6 - SHR Vx {, Vy}
                0x6 => {
                    // Set Vx = Vx SHR 1
                    chip8.registers[0xF] = chip8.registers[x as usize] & 0b00000001;
                    chip8.registers[x as usize] /= 2;
                }
                // 8xy7 - SUBN Vx, Vy
                0x7 => {
                    // Set Vx = Vy - Vx, set VF = NOT borrow
                    chip8.registers[0xF] =
                        if chip8.registers[y as usize] > chip8.registers[x as usize] {
                            1
                        } else {
                            0
                        };

                    chip8.registers[x as usize] =
                        match chip8.registers[y as usize].checked_sub(chip8.registers[x as usize]) {
                            Some(n) => n,
                            None => 0,
                        }
                }
                // 8xyE - SHL Vx {, Vy}
                0xE => {
                    // Set Vx = Vx SHL 1
                    chip8.registers[0xF] = if chip8.registers[x as usize] & 0b10000000 == 0b10000000
                    {
                        1
                    } else {
                        0
                    };
                    chip8.registers[x as usize] /= 2;
                }
                _ => {}
            }
        }
        // 9xy0 - SNE Vx, Vy
        0x9000 => {
            // Skip next instruction if Vx != Vy
            if chip8.registers[x as usize] != chip8.registers[y as usize] {
                chip8.pc += 2;
            }
        }
        // Annn - LD I, addr
        0xA000 => {
            // Set I = nnn
            chip8.register_i = nnn;
        }
        // Bnnn - JP V0, addr
        0xB000 => {
            // Jump to location nnn + V0
            chip8.pc = nnn + (chip8.registers[0] as u16);
        }
        // Cxkk - RND Vx, byte
        0xC000 => {
            // Set Vx = random byte AND kk
            chip8.registers[x as usize] = rand::random::<u8>() & kk;
        }
        // Dxyn - DRW Vx, Vy, nibble
        0xD000 => {
            // Display n-byte sprite starting at chip8.memory location I at (Vx, Vy), set VF = collision.
            /*
            The interpreter reads n bytes from chip8.memory,
            starting at the address stored in I.
            These bytes are then displayed as sprites on screen
            at coordinates (Vx, Vy). Sprites are XORed onto
            the existing screen. If this causes any pixels
            to be erased, VF is set to 1, otherwise it is
            set to 0. If the sprite is positioned so part of it
            is outside the coordinates of the display, it wraps
            around to the opposite side of the screen.
            */
            let start_x = chip8.registers[x as usize];
            let start_y = chip8.registers[y as usize];

            // Sprites are 8xN
            for line in 0..n {
                let sprite_line = chip8.memory[(chip8.register_i + line as u16) as usize];
                for column in 0..8 {
                    // wrap around with %
                    let pos_x = ((start_x % WIDTH) + column) % WIDTH;
                    let pos_y = ((start_y % HEIGHT) + line) % HEIGHT;
                    // println!("Pixel at {}({}),{}({})", pos_x, column, pos_y, line);

                    let px_index = (pos_y as usize) * 64 + (pos_x as usize);
                    let sprite_column_px = if (sprite_line << column) & 0b10000000 == 0b10000000 {
                        1
                    } else {
                        0
                    };
                    let old_px = chip8.display[px_index];
                    let new_px = old_px ^ sprite_column_px;
                    chip8.display[px_index] = new_px;

                    if old_px == 1 && new_px == 0 {
                        chip8.registers[0xF] = 1;
                    }
                }
            }
        }
        0xE000 => {
            match opcode & 0x00FF {
                // Ex9E - SKP Vx
                0x9E => {
                    // Skip next instruction if key with the value of Vx is pressed
                    if chip8.keys[chip8.registers[x as usize] as usize] {
                        chip8.pc += 2;
                    }
                }
                // ExA1 - SKNP Vx
                0xA1 => {
                    // Skip next instruction if key with the value of Vx is not pressed
                    if !chip8.keys[chip8.registers[x as usize] as usize] {
                        chip8.pc += 2;
                    }
                }
                _ => {}
            }
        }
        0xF000 => {
            match opcode & 0x00FF {
                // Fx07 - LD Vx, DT
                0x07 => {
                    // Set Vx = delay timer value
                    chip8.registers[x as usize] = chip8.timer_delay;
                }
                // Fx0A - LD Vx, K
                0x0A => {
                    // Wait for a key press, store the value of the key in Vx
                    // All execution stops until a key is pressed
                    chip8.hold_for_key = Some(x);
                }
                // Fx15 - LD DT, Vx
                0x15 => {
                    // Set delay timer = Vx
                    chip8.timer_delay = chip8.registers[x as usize];
                }
                // Fx18 - LD ST, Vx
                0x18 => {
                    // Set sound timer = Vx
                    chip8.timer_sound = chip8.registers[x as usize];
                }
                // Fx1E - ADD I, Vx
                0x1E => {
                    // Set I = I + Vx
                    chip8.register_i += chip8.registers[x as usize] as u16;
                }
                // Fx29 - LD F, Vx
                0x29 => {
                    // Set I = location of sprite for digit Vx
                    chip8.register_i = (chip8.registers[x as usize] * 5) as u16;
                }
                // Fx33 - LD B, Vx
                0x33 => {
                    // Store BCD representation of Vx in chip8.memory locations I, I+1, and I+2
                    // The interpreter takes the decimal value of Vx, and places the hundreds digit in chip8.memory at location in I, the tens digit at location I+1, and the ones digit at location I+2.
                    chip8.memory[(chip8.register_i + 0) as usize] =
                        chip8.registers[x as usize] / 100;
                    chip8.memory[(chip8.register_i + 1) as usize] =
                        (chip8.registers[x as usize] % 100) / 10;
                    chip8.memory[(chip8.register_i + 2) as usize] =
                        chip8.registers[x as usize] % 10;
                }
                // Fx55 - LD [I], Vx
                0x55 => {
                    // Store chip8.registers V0 through Vx in chip8.memory starting at location I
                    // The interpreter copies the values of chip8.registers V0 through Vx into chip8.memory, starting at the address in I
                    // I itself is left unmodified

                    for i in 0..=(x as usize) {
                        chip8.memory[(chip8.register_i as usize) + i] = chip8.registers[i];
                    }
                }
                // Fx65 - LD Vx, [I]
                0x65 => {
                    // Read chip8.registers V0 through Vx from chip8.memory starting at location I
                    // The interpreter reads values from chip8.memory starting at location I into chip8.registers V0 through Vx
                    // I itself is left unmodified

                    for i in 0..=(x as usize) {
                        chip8.registers[i] = chip8.memory[(chip8.register_i as usize) + i];
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn get_digit_sprites() -> [u8; 80] {
    {
        /*
        ****  11110000  0xF0
        *  *  10010000  0x90
        *  *  10010000  0x90
        *  *  10010000  0x90
        ****  11110000  0xF0

          *   00100000  0x20
         **   01100000  0x60
          *   00100000  0x20
          *   00100000  0x20
         ***  01110000  0x70

        ****  11110000  0xF0
           *  00010000  0x10
        ****  11110000  0xF0
        *     10000000  0x80
        ****  11110000  0xF0

        ****  11110000  0xF0
           *  00010000  0x10
        ****  11110000  0xF0
           *  00010000  0x10
        ****  11110000  0xF0

        *  *  10010000  0x90
        *  *  10010000  0x90
        ****  11110000  0xF0
           *  00010000  0x10
           *  00010000  0x10

        ****  11110000  0xF0
        *     10000000  0x80
        ****  11110000  0xF0
           *  00010000  0x10
        ****  11110000  0xF0

        ****  11110000  0xF0
        *     10000000  0x80
        ****  11110000  0xF0
        *  *  10010000  0x90
        ****  11110000  0xF0

        ****  11110000  0xF0
           *  00010000  0x10
          *   00100000  0x20
         *    01000000  0x40
         *    01000000  0x40

        ****  11110000  0xF0
        *  *  10010000  0x90
        ****  11110000  0xF0
        *  *  10010000  0x90
        ****  11110000  0xF0

        ****  11110000  0xF0
        *  *  10010000  0x90
        ****  11110000  0xF0
           *  00010000  0x10
        ****  11110000  0xF0

        ****  11110000  0xF0
        *  *  10010000  0x90
        ****  11110000  0xF0
        *  *  10010000  0x90
        *  *  10010000  0x90

        ***   11100000  0xE0
        *  *  10010000  0x90
        ***   11100000  0xE0
        *  *  10010000  0x90
        ***   11100000  0xE0

        ****  11110000  0xF0
        *     10000000  0x80
        *     10000000  0x80
        *     10000000  0x80
        ****  11110000  0xF0

        ***   11100000  0xE0
        *  *  10010000  0x90
        *  *  10010000  0x90
        *  *  10010000  0x90
        ***   11100000  0xE0

        ****  11110000  0xF0
        *     10000000  0x80
        ****  11110000  0xF0
        *     10000000  0x80
        ****  11110000  0xF0

        ****  11110000  0xF0
        *     10000000  0x80
        ****  11110000  0xF0
        *     10000000  0x80
        *     10000000  0x80
        */
    }

    return [
        0xF0, 0x90, 0x90, 0x90, 0xF0, 0x20, 0x60, 0x20, 0x20, 0x70, 0xF0, 0x10, 0xF0, 0x80, 0xF0,
        0xF0, 0x10, 0xF0, 0x10, 0xF0, 0x90, 0x90, 0xF0, 0x10, 0x10, 0xF0, 0x80, 0xF0, 0x10, 0xF0,
        0xF0, 0x80, 0xF0, 0x90, 0xF0, 0xF0, 0x10, 0x20, 0x40, 0x40, 0xF0, 0x90, 0xF0, 0x90, 0xF0,
        0xF0, 0x90, 0xF0, 0x10, 0xF0, 0xF0, 0x90, 0xF0, 0x90, 0x90, 0xE0, 0x90, 0xE0, 0x90, 0xE0,
        0xF0, 0x80, 0x80, 0x80, 0xF0, 0xE0, 0x90, 0x90, 0x90, 0xE0, 0xF0, 0x80, 0xF0, 0x80, 0xF0,
        0xF0, 0x80, 0xF0, 0x80, 0x80,
    ];
}

fn key_to_chip8_key_index(key: Key) -> Option<u8> {
    match key {
        Key::Key1 => Some(0x1),
        Key::Key2 => Some(0x2),
        Key::Key3 => Some(0x3),
        Key::Key4 => Some(0xC),
        Key::Q => Some(0x4),
        Key::W => Some(0x5),
        Key::E => Some(0x6),
        Key::R => Some(0xD),
        Key::A => Some(0x7),
        Key::S => Some(0x8),
        Key::D => Some(0x9),
        Key::F => Some(0xE),
        Key::Z => Some(0xA),
        Key::X => Some(0x0),
        Key::C => Some(0xB),
        Key::V => Some(0xF),
        _ => None,
    }
}

fn key_pressed(_app: &App, chip8: &mut Chip8, key: Key) {
    if let Some(key_index) = key_to_chip8_key_index(key) {
        if let Some(hold_for_key) = chip8.hold_for_key {
            chip8.registers[hold_for_key as usize] = key_index;
        }
        chip8.keys[key_index as usize] = true;
    }
}

fn key_released(_app: &App, chip8: &mut Chip8, key: Key) {
    if let Some(key_index) = key_to_chip8_key_index(key) {
        chip8.keys[key_index as usize] = false;
    }
}

#[allow(dead_code)]
fn load_rom_from_file(filepath: &str) -> Vec<u8> {
    match std::fs::read(filepath) {
        Ok(instructions) => instructions,
        Err(err) => {
            println!("Error reading ROM at {} : {}", filepath, err);
            panic!();
        }
    }
}
