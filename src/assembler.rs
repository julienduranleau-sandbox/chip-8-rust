use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

const DEBUG: bool = false;

struct AsmLine {
    line: String,
    opcode: Option<u16>,
    memory_position: u16,
}
enum OpcodeError {
    NoOpcode,
    Incomplete,
}
enum AddressError {
    UnknownLabel,
}

#[allow(dead_code)]
pub fn assemble(filename: &str) -> Vec<u8> {
    let mut instructions: Vec<u8> = vec![];

    let file = File::open(filename).unwrap();
    let reader = BufReader::new(file);
    let mut labels: HashMap<String, u16> = HashMap::new();

    let mut asm_lines: Vec<AsmLine> = vec![];
    let mut memory_position = 0x200;

    for (_index, line) in reader.lines().enumerate() {
        if let Ok(line) = line {
            if let Ok(opcode) = parse_asm_line(&line, &mut labels, memory_position) {
                asm_lines.push(AsmLine {
                    line,
                    opcode: Some(opcode),
                    memory_position,
                });
                memory_position += 2;
            } else {
                asm_lines.push(AsmLine {
                    line,
                    opcode: None,
                    memory_position,
                });
                memory_position += 2;
            }
        }
    }

    for mut asm_line in asm_lines {
        if asm_line.opcode.is_none() {
            if let Ok(opcode) =
                parse_asm_line(&asm_line.line, &mut labels, asm_line.memory_position)
            {
                asm_line.opcode = Some(opcode);
            } else {
                panic!("Invalid asm line: {}", asm_line.line);
            }
        }

        instructions.push(((asm_line.opcode.unwrap() & 0xFF00) >> 8) as u8);
        instructions.push((asm_line.opcode.unwrap() & 0x00FF) as u8);
    }

    instructions
}

#[allow(dead_code)]
fn parse_asm_line(
    line: &String,
    labels: &mut HashMap<String, u16>,
    memory_index: u16,
) -> Result<u16, OpcodeError> {
    // Strip comments
    let line = match line.find(';') {
        Some(i) => String::from(&line[..i]),
        None => String::from(line),
    };

    // Return early for empty lines
    if line.is_empty() {
        return Err(OpcodeError::NoOpcode);
    }

    let parts: Vec<&str> = line
        .trim_start_matches(" ")
        .trim_end_matches(" ")
        .split(" ")
        .collect();

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
    let kk = if parts.len() >= 3 && y.is_empty() {
        match get_hex_str(parts[2]) {
            Some(n) => format!("{:0>2}", n),
            None => String::default(),
        }
    } else {
        String::default()
    };
    let nnn = if x.is_empty() && y.is_empty() && parts.len() == 2 {
        match get_address(parts[1], &labels) {
            Ok(v) => v,
            Err(_e) => return Err(OpcodeError::Incomplete),
        }
    } else {
        String::default()
    };
    let n = if parts.len() >= 4 {
        match get_hex_str(parts[3]) {
            Some(n) => format!("{}", n),
            None => String::default(),
        }
    } else {
        String::default()
    };

    // Label
    if command.ends_with(":") {
        let label_name = String::from(&command[..(command.len() - 1)]);
        if DEBUG {
            println!("Insert {} with value {:#x}", label_name, memory_index);
        }
        labels.insert(label_name, memory_index);
        return Err(OpcodeError::NoOpcode);
    }

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
                let addr = match get_address(parts[2], &labels) {
                    Ok(v) => v,
                    Err(_e) => return Err(OpcodeError::Incomplete),
                };
                Some(format!("B{:0>2}", addr))
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
            // Vx, Vy
            if !x.is_empty() && !y.is_empty() {
                Some(format!("8{}{}0", x, y))
            // I, Vx
            } else if parts[1] == "I," && !y.is_empty() {
                Some(format!("F{}55", y))
            // I, addr
            } else if parts[1] == "I," {
                let addr = match get_address(parts[2], &labels) {
                    Ok(v) => v,
                    Err(_e) => return Err(OpcodeError::Incomplete),
                };
                Some(format!("A{}", addr))
            // Vx, DT
            } else if !x.is_empty() && parts[2] == "DT" {
                Some(format!("F{}07", x))
            // Vx, K
            } else if !x.is_empty() && parts[2] == "K" {
                Some(format!("F{}0A", x))
            // DT, Vx
            } else if parts[1] == "DT," && !y.is_empty() {
                Some(format!("F{}15", y))
            // ST, Vx
            } else if parts[1] == "ST," && !y.is_empty() {
                Some(format!("F{}18", y))
            // F, Vx
            } else if parts[1] == "F," && !y.is_empty() {
                Some(format!("F{}29", y))
            // B, Vx
            } else if parts[1] == "B," && !y.is_empty() {
                Some(format!("F{}33", y))
            // Vx, I
            } else if !x.is_empty() && parts[2] == "I" {
                Some(format!("F{}65", x))
            // Vx, byte
            } else if !x.is_empty() && !kk.is_empty() {
                Some(format!("6{}{}", x, kk))
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
        "SHR" => {
            if y.is_empty() {
                Some(format!("8{}06", x))
            } else {
                Some(format!("8{}{}6", x, y))
            }
        }
        // 8xy7 - SUBN Vx, Vy
        "SUBN" => Some(format!("8{}{}7", x, y)),
        // 8xyE - SHL Vx {, Vy}
        "SHL" => {
            if y.is_empty() {
                Some(format!("8{}0E", x))
            } else {
                Some(format!("8{}{}E", x, y))
            }
        }
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
            if DEBUG {
                println!("Opcode: {:#06x}", opcode);
            }
            Ok(opcode)
        } else {
            println!("{}", line);
            println!("x: {}", x);
            println!("y: {}", y);
            println!("n: {}", n);
            println!("kk: {}", kk);
            println!("nnn: {}", nnn);
            println!("Wrong opcode format : {}", opcode_str);
            panic!();
        }
    } else {
        return Err(OpcodeError::NoOpcode);
    }
}

fn get_address(text: &str, labels: &HashMap<String, u16>) -> Result<String, AddressError> {
    if text.starts_with("0x") {
        Ok(format!("{:0>3}", text.trim_start_matches("0x")))
    } else {
        match labels.get(text) {
            Some(label_value) => Ok(String::from(
                format!("{:#03x}", label_value).trim_start_matches("0x"),
            )),
            None => Err(AddressError::UnknownLabel),
        }
    }
}

fn get_hex_str(text: &str) -> Option<String> {
    if text.starts_with("0x") {
        Some(String::from(text.trim_start_matches("0x")))
    } else {
        if let Ok(n) = u16::from_str_radix(text, 10) {
            Some(String::from(format!("{:#x}", n).trim_start_matches("0x")))
        } else {
            None
        }
    }
}

#[test]
fn test_parse_asm_line() {
    let mut labels: HashMap<String, u16> = HashMap::new();
    // Default tests
    #[cfg_attr(rustfmt, rustfmt_skip)]
    {
        assert_eq!(parse_asm_line(String::from("SYS 0xFE9"), &mut labels, 0x200), Some(0x0FE9));
        assert_eq!(parse_asm_line(String::from("CLS"), &mut labels, 0x200), Some(0x00E0));
        assert_eq!(parse_asm_line(String::from("RET"), &mut labels, 0x200), Some(0x00EE));
        assert_eq!(parse_asm_line(String::from("JP 0xE13"), &mut labels, 0x200), Some(0x1E13));
        assert_eq!(parse_asm_line(String::from("CALL 0x5C1"), &mut labels, 0x200), Some(0x25C1));
        assert_eq!(parse_asm_line(String::from("SE V5, 0xFE"), &mut labels, 0x200), Some(0x35FE));
        assert_eq!(parse_asm_line(String::from("SNE VC, 0xD1"), &mut labels, 0x200), Some(0x4CD1));
        assert_eq!(parse_asm_line(String::from("SE V1, VF"), &mut labels, 0x200), Some(0x51F0));
        assert_eq!(parse_asm_line(String::from("LD VD, 0x92"), &mut labels, 0x200), Some(0x6D92));
        assert_eq!(parse_asm_line(String::from("ADD V0, 0xFF"), &mut labels, 0x200), Some(0x70FF));
        assert_eq!(parse_asm_line(String::from("LD V0, V3"), &mut labels, 0x200), Some(0x8030));
        assert_eq!(parse_asm_line(String::from("OR V1, V2"), &mut labels, 0x200), Some(0x8121));
        assert_eq!(parse_asm_line(String::from("AND V5, V1"), &mut labels, 0x200), Some(0x8512));
        assert_eq!(parse_asm_line(String::from("XOR V2, VA"), &mut labels, 0x200), Some(0x82A3));
        assert_eq!(parse_asm_line(String::from("ADD VC, VF"), &mut labels, 0x200), Some(0x8CF4));
        assert_eq!(parse_asm_line(String::from("SUB V0, V8"), &mut labels, 0x200), Some(0x8085));
        assert_eq!(parse_asm_line(String::from("SHR V1"), &mut labels, 0x200), Some(0x8106));
        assert_eq!(parse_asm_line(String::from("SHR V1 VC"), &mut labels, 0x200), Some(0x81C6));
        assert_eq!(parse_asm_line(String::from("SUBN VA, V6"), &mut labels, 0x200), Some(0x8A67));
        assert_eq!(parse_asm_line(String::from("SHL V2"), &mut labels, 0x200), Some(0x820E));
        assert_eq!(parse_asm_line(String::from("SHL V2 V1"), &mut labels, 0x200), Some(0x821E));
        assert_eq!(parse_asm_line(String::from("SNE V0, VE"), &mut labels, 0x200), Some(0x90E0));
        assert_eq!(parse_asm_line(String::from("LD I, 0x46E"), &mut labels, 0x200), Some(0xA46E));
        assert_eq!(parse_asm_line(String::from("JP V0, 0xF12"), &mut labels, 0x200), Some(0xBF12));
        assert_eq!(parse_asm_line(String::from("RND V4, 0xBC"), &mut labels, 0x200), Some(0xC4BC));
        assert_eq!(parse_asm_line(String::from("DRW V5, VF, 0xC"), &mut labels, 0x200), Some(0xD5FC));
        assert_eq!(parse_asm_line(String::from("SKP V5"), &mut labels, 0x200), Some(0xE59E));
        assert_eq!(parse_asm_line(String::from("SKNP VF"), &mut labels, 0x200), Some(0xEFA1));
        assert_eq!(parse_asm_line(String::from("LD VA, DT"), &mut labels, 0x200), Some(0xFA07));
        assert_eq!(parse_asm_line(String::from("LD VA, K"), &mut labels, 0x200), Some(0xFA0A));
        assert_eq!(parse_asm_line(String::from("LD DT, V4"), &mut labels, 0x200), Some(0xF415));
        assert_eq!(parse_asm_line(String::from("LD ST, V4"), &mut labels, 0x200), Some(0xF418));
        assert_eq!(parse_asm_line(String::from("ADD I, VF"), &mut labels, 0x200), Some(0xFF1E));
        assert_eq!(parse_asm_line(String::from("LD F, VC"), &mut labels, 0x200), Some(0xFC29));
        assert_eq!(parse_asm_line(String::from("LD B, VB"), &mut labels, 0x200), Some(0xFB33));
        assert_eq!(parse_asm_line(String::from("LD I, VD"), &mut labels, 0x200), Some(0xFD55));
        assert_eq!(parse_asm_line(String::from("LD VC, I"), &mut labels, 0x200), Some(0xFC65));

        // Edge cases
        assert_eq!(parse_asm_line(String::from("LD VA, 0x2"), &mut labels, 0x200), Some(0x6A02));
        assert_eq!(parse_asm_line(String::from("CLS ; some comments"), &mut labels, 0x200), Some(0x00E0));
        assert_eq!(parse_asm_line(String::from(";LD VA, 0x2"), &mut labels, 0x200), None);
        assert_eq!(parse_asm_line(String::from("some_label:"), &mut labels, 0x200), None);
    }
}
