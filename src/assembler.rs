use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn assemble(filename: &str) -> Vec<u8> {
    let instructions: Vec<u8> = vec![];

    let file = File::open(filename).unwrap();
    let reader = BufReader::new(file);

    for (_index, line) in reader.lines().enumerate() {
        if let Ok(line) = line {
            println!("{}", line);
            let parts: Vec<&str> = line.split(" ").collect();
            println!("{:?}", parts);

            let command = parts[0];
            let x = if parts.len() >= 2 && parts[1].starts_with('V') {
                parts[1].trim_start_matches("V").trim_end_matches(",")
            } else {
                ""
            };
            let y = if parts.len() >= 3 && parts[2].starts_with('V') {
                parts[2].trim_start_matches("V").trim_end_matches(",")
            } else {
                ""
            };
            let kk = if y.is_empty() && parts.len() >= 3 {
                format!("{:0>2}", parts[2])
            } else {
                String::default()
            };
            let nnn = if x.is_empty() && y.is_empty() && parts.len() >= 2 {
                format!("{:0>3}", parts[1])
            } else {
                String::default()
            };
            let n = if parts.len() >= 4 {
                format!("{}", parts[3])
            } else {
                String::default()
            };

            let opcode_str = match command {
                // 00E0 - CLS
                "CLS" => Some(String::from("00E0")),
                // 00EE - RET
                "RET" => Some(String::from("00EE")),
                // 0nnn - SYS addr
                "SYS" => Some(format!("0{}", nnn)),
                // 1nnn - JP addr
                // Bnnn - JP V0, addr
                "JP" => {
                    if parts.len() == 2 {
                        Some(format!("1{}", nnn))
                    } else {
                        Some(format!("B{:0>2}", parts[2]))
                    }
                }
                // 2nnn - CALL addr
                "CALL" => Some(format!("2{}", nnn)),
                // 3xkk - SE Vx, byte
                // 5xy0 - SE Vx, Vy
                "SE" => {
                    // Vx, byte
                    if y.is_empty() {
                        Some(format!("3{}{}", x, kk))
                    // Vx, Vy
                    } else {
                        Some(format!("5{}{}0", x, y))
                    }
                }
                // 4xkk - SNE Vx, byte
                // 9xy0 - SNE Vx, Vy
                "SNE" => {
                    // Vx, byte
                    if y.is_empty() {
                        Some(format!("4{}{}", x, kk))
                    // Vx, Vy
                    } else {
                        Some(format!("9{}{}0", x, y))
                    }
                }
                // 6xkk - LD Vx, byte
                // 8xy0 - LD Vx, Vy
                // Annn - LD I, addr
                // Fx07 - LD Vx, DT
                // Fx0A - LD Vx, K
                // Fx15 - LD DT, Vx
                // Fx18 - LD ST, Vx
                // Fx29 - LD F, Vx
                // Fx33 - LD B, Vx
                // Fx55 - LD [I], Vx
                // Fx65 - LD Vx, [I]
                "LD" => {
                    // Vx, byte
                    if !x.is_empty() && !kk.is_empty() {
                        Some(format!("6{}{}", x, kk))
                    // Vx, Vy
                    } else if !x.is_empty() && !y.is_empty() {
                        Some(format!("8{}{}0", x, y))
                    // I, addr
                    } else if parts[1] == "I" && !nnn.is_empty() {
                        Some(format!("A{}", nnn))
                    // Vx, DT
                    } else if !x.is_empty() && parts[2] == "DT" {
                        Some(format!("F{}07", x))
                    // Vx, K
                    } else if !x.is_empty() && parts[2] == "K" {
                        Some(format!("F{}0A", x))
                    // DT, Vx
                    } else if parts[1] == "DT" && !y.is_empty() {
                        Some(format!("F{}15", y))
                    // ST, Vx
                    } else if parts[1] == "ST" && !y.is_empty() {
                        Some(format!("F{}18", y))
                    // F, Vx
                    } else if parts[1] == "F" && !y.is_empty() {
                        Some(format!("F{}29", y))
                    // B, Vx
                    } else if parts[1] == "B" && !y.is_empty() {
                        Some(format!("F{}33", y))
                    // I, Vx
                    } else if parts[1] == "I" && !y.is_empty() {
                        Some(format!("F{}55", y))
                    // Vx, I
                    } else if !x.is_empty() && parts[2] == "I" {
                        Some(format!("F{}65", y))
                    } else {
                        None
                    }
                }
                // 7xkk - ADD Vx, byte
                // 8xy4 - ADD Vx, Vy
                // Fx1E - ADD I, Vx
                "ADD" => {
                    // Vx, byte
                    if y.is_empty() {
                        Some(format!("7{}{}", x, kk))
                    // I, Vx
                    } else if x.is_empty() {
                        // use y since it's the 2nd param
                        Some(format!("F{}1E", y))
                    // Vx, Vy
                    } else {
                        Some(format!("8{}{}4", x, y))
                    }
                }
                // 8xy1 - OR Vx, Vy
                "OR" => Some(format!("8{}{}1", x, y)),
                // 8xy2 - AND Vx, Vy
                "AND" => Some(format!("8{}{}2", x, y)),
                // 8xy3 - XOR Vx, Vy
                "XOR" => Some(format!("8{}{}3", x, y)),
                // 8xy5 - SUB Vx, Vy
                "SUB" => Some(format!("8{}{}5", x, y)),
                // 8xy6 - SHR Vx {, Vy}
                "SHR" => Some(format!("8{}{}6", x, y)),
                // 8xy7 - SUBN Vx, Vy
                "SUBN" => Some(format!("8{}{}7", x, y)),
                // 8xyE - SHL Vx {, Vy}
                "SHL" => Some(format!("8{}{}E", x, y)),
                // Cxkk - RND Vx, byte
                "RND" => Some(format!("C{}{}", x, kk)),
                // Dxyn - DRW Vx, Vy, nibble
                "DRW" => Some(format!("D{}{}{}", x, y, n)),
                // Ex9E - SKP Vx
                "SKP" => Some(format!("E{}9E", x)),
                // ExA1 - SKNP Vx
                "SKNP" => Some(format!("E{}A1", x)),
                _ => None,
            };
            if let Some(opcode_str) = opcode_str {
                if let Ok(opcode) = u16::from_str_radix(opcode_str.as_str(), 16) {
                    println!("Opcode: {:#06x}", opcode);
                } else {
                    println!("Wrong opcode format : {}", opcode_str);
                    panic!();
                }
            }
        }
    }

    instructions
}
