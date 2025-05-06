use std::fs;

#[derive(Debug)]
struct CPU {
    registers: [u8; 16],
    position_in_memory: usize,
    memory: [u8; 0x1000],
    stack: [u16; 16],
    stack_pointer: usize,
    display: Display,
}

impl CPU {
    fn read_opcode(&self) -> u16 {
        let p = self.position_in_memory;
        let op_byte1 = self.memory[p] as u16;
        let op_byte2 = self.memory[p + 1] as u16;

        op_byte1 << 8 | op_byte2
    }

    fn run(&mut self) {
        loop {
            let opcode = self.read_opcode();
            println!("instruction: {:x}", opcode);
            self.position_in_memory += 2;

            let c = ((opcode >> 12) & 0x000F) as u8;
            let x = ((opcode >> 8) & 0x000F) as u8;
            let y = ((opcode >> 4) & 0x000F) as u8;
            let d = (opcode & 0x000F) as u8;

            let nnn = opcode & 0x0FFF;
            let kk = (opcode & 0x00FF) as u8;

            match (c, x, y, d) {
                (0, 0, 0, 0) => {
                    return;
                }
                (0, 0, 0xE, 0) => self.display.clear(),
                (0, 0, 0xE, 0xE) => self.ret(),
                (0x1, _, _, _) => self.jmp_to_addr(nnn),
                (0x2, _, _, _) => self.call(nnn),
                (0x3, _, _, _) => self.skip_if_eq(x, kk),
                (0x4, _, _, _) => self.skip_if_neq(x, kk),
                (0x5, _, _, 0) => self.skip_if_eq_registers(x, y),
                (0x6, _, _, _) => self.load_in_register(x, kk),
                (0x7, _, _, _) => self.add_xkk(x, kk),
                (0x8, _, _, 0x4) => self.add_xy(x, y),
                _ => todo!("opcode: {:04x}", opcode),
            }
        }
    }

    fn call(&mut self, addr: u16) {
        let sp = self.stack_pointer;
        let stack = &mut self.stack;

        if sp > stack.len() {
            panic!("Stack overflow");
        }

        stack[sp] = self.position_in_memory as u16;
        self.stack_pointer += 1;
        self.position_in_memory = addr as usize;
    }

    fn ret(&mut self) {
        if self.stack_pointer == 0 {
            panic!("Stack underflow");
        }

        self.stack_pointer -= 1;
        let call_addr = self.stack[self.stack_pointer];
        self.position_in_memory = call_addr as usize;
    }

    fn jmp_to_addr(&mut self, addr: u16) {
        self.position_in_memory = addr as usize;
    }

    fn skip_if_eq(&mut self, x: u8, kk: u8) {
        if self.registers[x as usize] == kk {
            self.position_in_memory += 2;
        }
    }

    fn skip_if_neq(&mut self, r: u8, kk: u8) {
        if self.registers[r as usize] != kk {
            self.position_in_memory += 2;
        }
    }

    fn skip_if_eq_registers(&mut self, x: u8, y: u8) {
        if self.registers[x as usize] == self.registers[y as usize] {
            self.position_in_memory += 2;
        }
    }

    fn load_in_register(&mut self, x: u8, kk: u8) {
        self.registers[x as usize] = kk;
        println!("load in reg: {}", kk);
        println!("reg[{}] = {}", x, self.registers[x as usize]);
    }

    fn add_xkk(&mut self, x: u8, kk: u8) {
        let arg1 = self.registers[x as usize];

        self.registers[x as usize] = arg1 + kk;
    }

    fn add_xy(&mut self, x: u8, y: u8) {
        let arg1 = self.registers[x as usize];
        let arg2 = self.registers[y as usize];

        let (val, overflow) = arg1.overflowing_add(arg2);
        self.registers[x as usize] = val;

        if overflow {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }
    }
}

#[derive(Debug)]
struct Display {
    pixels: [[bool; 32]; 64],
}

impl Display {
    fn new() -> Self {
        Self {
            pixels: [[false; 32]; 64],
        }
    }

    fn clear(&mut self) {
        self.pixels
            .iter_mut()
            .for_each(|r| r.iter_mut().for_each(|v| *v = false));
    }
}

fn main() {
    let mut cpu = CPU {
        registers: [0; 16],
        memory: [0; 4096],
        position_in_memory: 512,
        stack: [0; 16],
        stack_pointer: 0,
        display: Display::new(),
    };

    //let program = fs::read("add.ch8").unwrap();
    let program = fs::read("cond.ch8").unwrap();
    let program_len = program.len();

    let mem = &mut cpu.memory;

    mem[512..512 + program_len].copy_from_slice(&program);

    cpu.run();

    println!("{}", cpu.registers[0]);
}
