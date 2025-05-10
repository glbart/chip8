use anyhow::{Context, Result, *};
use clap::Parser;
use minifb::{Window, WindowOptions};
use std::fs;

#[derive(Parser, Debug)]
#[command(name = "CHIP8 emulator", about = "A simple chip8 emulator on rust")]
struct Cli {
    /// Path to the program (in binary format)
    #[arg(short, long)]
    file: std::path::PathBuf,
}

#[derive(Debug)]
struct CPU {
    registers: [u8; 16],
    register_I: u16,
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
        let mut i = 0;
        loop {
            i += 1;
            if i > 100 {
                break;
            }
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
                (0x8, _, _, 0x0) => self.set_xy(x, y),
                (0x8, _, _, 0x1) => self.or_xy(x, y),
                (0x8, _, _, 0x2) => self.and_xy(x, y),
                (0x8, _, _, 0x3) => self.xor_xy(x, y),
                (0x8, _, _, 0x4) => self.add_xy(x, y),
                (0x8, _, _, 0x5) => self.sub_xy(x, y),
                (0x8, _, _, 0x6) => self.shr_x(x),
                (0x8, _, _, 0x7) => self.subn_xy(x, y),
                (0x8, _, _, 0xE) => self.shl_x(x),
                (0x9, _, _, 0x0) => self.skip_if_neq_registers(x, y),
                (0xA, _, _, _) => self.set_I(nnn),
                (0xB, _, _, _) => self.jmp_to_addr_x(x, nnn),
                (0xC, _, _, _) => self.set_rand_x(x, kk),
                (0xD, _, _, _) => self.draw(x, y, d),
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
        println!("jump to addr: {:x}", addr);
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

    fn set_xy(&mut self, x: u8, y: u8) {
        self.registers[x as usize] = self.registers[y as usize];
    }

    fn or_xy(&mut self, x: u8, y: u8) {
        let arg1 = self.registers[x as usize];
        let arg2 = self.registers[y as usize];
        self.registers[x as usize] = arg1 | arg2;
    }

    fn and_xy(&mut self, x: u8, y: u8) {
        let arg1 = self.registers[x as usize];
        let arg2 = self.registers[y as usize];
        self.registers[x as usize] = arg1 & arg2;
    }

    fn xor_xy(&mut self, x: u8, y: u8) {
        let arg1 = self.registers[x as usize];
        let arg2 = self.registers[y as usize];
        self.registers[x as usize] = arg1 ^ arg2;
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

    fn sub_xy(&mut self, x: u8, y: u8) {
        let arg1 = self.registers[x as usize];
        let arg2 = self.registers[y as usize];
        self.registers[x as usize] = arg1 - arg2;

        if arg1 > arg2 {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }
    }

    fn shr_x(&mut self, x: u8) {
        let val_x = self.registers[x as usize];
        self.registers[x as usize] >>= 1;

        self.registers[0xF] = val_x & 1;
    }

    fn subn_xy(&mut self, x: u8, y: u8) {
        let arg1 = self.registers[x as usize];
        let arg2 = self.registers[y as usize];
        self.registers[x as usize] = arg2 - arg1;

        if arg2 > arg1 {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }
    }

    fn shl_x(&mut self, x: u8) {
        let val_x = self.registers[x as usize];
        self.registers[x as usize] <<= 1;

        self.registers[0xF] = val_x >> 7;
    }

    fn skip_if_neq_registers(&mut self, x: u8, y: u8) {
        if self.registers[x as usize] != self.registers[y as usize] {
            self.position_in_memory += 2;
        }
    }

    fn set_I(&mut self, addr: u16) {
        self.register_I = addr;
    }

    fn jmp_to_addr_x(&mut self, x: u8, addr: u16) {
        self.position_in_memory = (addr + (self.registers[x as usize] as u16)) as usize;
    }

    fn set_rand_x(&mut self, x: u8, kk: u8) {
        self.registers[x as usize] = 1 & kk;
    }

    fn draw(&mut self, ix: u8, iy: u8, n: u8) {
        println!("start draw");
        let start_x: usize = (self.registers[ix as usize] % 64).into();
        let start_y: usize = (self.registers[iy as usize] % 32).into();
        println!("x: {}, y: {}", start_x, start_y);
        self.registers[0xF] = 0;

        let pixels = &mut self.display.pixels;

        for i in 0..n as usize {
            let y: usize = start_y + i;
            let sprite = self.memory[(self.register_I + i as u16) as usize];
            for j in 0..8 {
                let x: usize = start_x + j;
                let p = sprite & (1 << (7 - j));
                println!("bit: {} | {:b} | {:b}", j, sprite, p);
                if p > 0 && pixels[y][x] {
                    pixels[y][x] = false;
                    self.registers[0xF] = 1;
                } else if (p == 0 && pixels[y][x]) || (p > 0 && !pixels[y][x])  {
                    pixels[y][x] = true;
                }
            }
        }
    }
}

#[derive(Debug)]
struct Display {
    pixels: [[bool; 64]; 32],
}

impl Display {
    fn new() -> Self {
        Self {
            pixels: [[false; 64]; 32],
        }
    }

    fn clear(&mut self) {
        // self.pixels
        // .iter_mut()
        //.for_each(|r| r.iter_mut().for_each(|v| *v = false));
    }
}

const BASE_WIDTH: usize = 640;
const BASE_HEIGHT: usize = 320;
const PADDING: usize = 30;
const WIDTH: usize = PADDING + BASE_WIDTH + PADDING;
const HEIGHT: usize = PADDING + BASE_HEIGHT + PADDING;

fn main() -> Result<()> {
    let args = Cli::parse();

    let program = fs::read(&args.file)
        .with_context(|| format!("Couldn't read program `{}`", &args.file.display()))?;
    let program_len = program.len();

    if program_len == 0 {
        return Err(anyhow!("Program don't contains code!!!"));
    }

    let mut cpu = CPU {
        registers: [0; 16],
        register_I: 0,
        memory: [0; 4096],
        position_in_memory: 512,
        stack: [0; 16],
        stack_pointer: 0,
        display: Display::new(),
    };

    let mem = &mut cpu.memory;

    mem[512..512 + program_len].copy_from_slice(&program);

    cpu.run();

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut window = Window::new("CHIP8", WIDTH, HEIGHT, WindowOptions::default())
        .with_context(|| "Couldn't create window".to_string())?;

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        for (i, p) in buffer.iter_mut().enumerate() {
            let row = i / WIDTH;
            let col = i % WIDTH + 1;
            let inner_row: i32 = row as i32 - 30;
            let inner_col: i32 = col as i32 - 30;
            let virtual_row = inner_row / 10;
            let virtual_col = inner_col / 10;
            if row < PADDING || row >= PADDING + BASE_HEIGHT {
                *p = 0x252429;
            } else if col < PADDING || col >= PADDING + BASE_WIDTH {
                *p = 0x252429;
            } else if virtual_row < 32
                && virtual_col < 64
                && cpu.display.pixels[virtual_row as usize][virtual_col as usize]
            {
                *p = 0xFFFFFF;
            }
        }

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }

    println!("{}", cpu.registers[0]);

    Ok(())
}
