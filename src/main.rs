#![allow(dead_code)]
extern crate sdl2;
#[macro_use]
extern crate maplit;

use rand::Rng;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::fs;

use sdl2::rect::Rect;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

struct Chip8 {
    registers: [u8; 16],
    memory: Vec<u8>,
    stack: Vec<u16>,
    i: u16,
    dt: u8,
    st: u8,
    pc: u16,
    display: [[u8; 64]; 32],
    cycle_count: i32,
    keys_pressed: [bool; 16],
}

impl Chip8 {
    fn new() -> Chip8 {
        let mut x = Chip8 {
            registers: [0u8; 16],
            memory: vec![],
            stack: vec![],
            i: 0u16,
            dt: 0u8,
            st: 0u8,
            pc: 512u16,
            display: [[0u8; 64]; 32],
            cycle_count: 0,
            keys_pressed: [false; 16],
        };

        // Load the digit sprites into memory starting at 0x00. They're each 5 bytes
        // long. They're in order, from 0 through F.
        let mut digits: Vec<u8> = vec![
            0xF0, 0x90, 0x90, 0x90, 0xF0, 0x20, 0x60, 0x20, 0x20, 0x70, 0xF0, 0x10, 0xF0, 0x80,
            0xF0, 0xF0, 0x10, 0xF0, 0x10, 0xF0, 0x90, 0x90, 0xF0, 0x10, 0x10, 0xF0, 0x80, 0xF0,
            0x10, 0xF0, 0xF0, 0x80, 0xF0, 0x90, 0xF0, 0xF0, 0x10, 0x20, 0x40, 0x40, 0xF0, 0x90,
            0xF0, 0x90, 0xF0, 0xF0, 0x90, 0xF0, 0x10, 0xF0, 0xF0, 0x90, 0xF0, 0x90, 0x90, 0xE0,
            0x90, 0xE0, 0x90, 0xE0, 0xF0, 0x80, 0x80, 0x80, 0xF0, 0xE0, 0x90, 0x90, 0x90, 0xE0,
            0xF0, 0x80, 0xF0, 0x80, 0xF0, 0xF0, 0x80, 0xF0, 0x80, 0x80,
        ];

        x.memory.append(&mut digits);

        // pad out to 512 bytes, which is where programs expect to load from
        x.memory.append(&mut vec![0; 512 - x.memory.len()]);

        return x;
    }

    fn load_application(&mut self, path: &str) {
        let mut x = match fs::read(path) {
            Ok(file) => file,
            Err(error) => panic!("Problem opening file! Error: {:?}", error),
        };

        self.memory.append(&mut x);

        // pad out to 4K memory (ought to be enough for anybody)
        self.memory
            .append(&mut vec![0; (2usize).pow(12) - self.memory.len()]);
    }

    // Since the Chip8 keyboard is hexadecimal, we just accept a list
    // of integers from 0x1 to 0xF, and use them to index into an arary.
    fn register_keydown(&mut self, keys: impl Iterator<Item = u8>) {
        self.keys_pressed = [false; 16];
        for k in keys {
            self.keys_pressed[k as usize] = true;
        }
    }

    fn get_pretty_debug_info(&mut self) -> String {
        let mut output = String::from("registers | ");

        output.push_str(
            &(0..15)
                .into_iter()
                .map(|x| format!("{:2X}", x))
                .collect::<Vec<String>>()
                .join(" | "),
        );

        output.push_str("\n");
        output.push_str("values    | ");

        output.push_str(
            &self
                .registers
                .into_iter()
                .map(|x| format!("{:2x}", x))
                .collect::<Vec<String>>()
                .join(" | "),
        );

        output.push_str(&format!("\nRegister I: {:4x}", self.i));

        output.push_str(&format!(
            "\nPC: {2:x} DT: {:3} ST: {:3}",
            self.pc, self.dt, self.st
        ));

        return output;
    }

    fn execute_next_instruction(&mut self) {
        // combine two bytes to get the full opcode, as a u16
        let opcode = (self.memory[self.pc as usize] as u16) << 8
            | self.memory[(self.pc + 1) as usize] as u16;

        // println!("{:X}", opcode);
        // println!("pc: {} i: {} dt: {}", self.pc, self.i, self.dt);
        // println!("{:?}", self.registers);
        self.execute_opcode(opcode);

        self.pc += 2;
    }

    fn execute_opcode(&mut self, opcode: u16) {
        // "first" means the most significant
        let first_nibble = (opcode >> 12) & 0xf;
        let second_nibble = ((opcode >> 8) & 0xf) as usize;
        let third_nibble = ((opcode >> 4) & 0xf) as usize;
        let last_nibble = (opcode & 0xf) as u8;
        let last_two_nibles = (opcode & 0xff) as u8;
        let last_three_nibbles = (opcode & 0xfff) as u16;

        match first_nibble {
            0x0 => match opcode {
                0x00E0 => self.clear_screen(),
                0x00EE => self.return_from_submodule(),
                _ => panic!("Unrecognized opcode: {:X}", opcode),
            },
            0x1 => self.jump(last_three_nibbles),
            0x2 => self.call(last_three_nibbles),
            0x3 => self.skip_if_reg_equal_to_val(second_nibble, last_two_nibles),
            0x4 => self.skip_if_reg_not_equal_to_val(second_nibble, last_two_nibles),
            0x5 => self.skip_if_reg_equal_to_reg(second_nibble, third_nibble),
            0x6 => self.load_value_into_reg(second_nibble, last_two_nibles),
            0x7 => self.add_value_to_reg(second_nibble, last_two_nibles),
            0x8 => match last_nibble {
                0x0 => self.load_reg_into_reg(second_nibble, third_nibble),
                0x1 => self.or_reg(second_nibble, third_nibble),
                0x2 => self.and_reg(second_nibble, third_nibble),
                0x3 => self.xor_reg(second_nibble, third_nibble),
                0x4 => self.add_reg(second_nibble, third_nibble),
                0x5 => self.sub_reg(second_nibble, third_nibble),
                0x6 => self.shr_reg(second_nibble),
                0x7 => self.subn_reg(second_nibble, third_nibble),
                0xE => self.shl_reg(second_nibble),
                _ => panic!("Unrecognized opcode: {:X}", opcode),
            },
            0x9 => self.skip_next_if_reg_equal_reg(second_nibble, third_nibble),
            0xA => self.load_value_into_i(last_three_nibbles),
            0xB => self.jump_to_reg_zero(last_three_nibbles),
            0xC => self.random(second_nibble, last_two_nibles),
            0xD => self.draw_sprite(second_nibble, third_nibble, last_nibble),
            0xE => match last_two_nibles {
                0x9E => self.skip_next_if_key_pressed(second_nibble),
                0xA1 => self.skip_next_if_key_not_pressed(second_nibble),
                _ => panic!("Unrecognized opcode: {:X}", opcode),
            },
            0xF => match last_two_nibles {
                0x07 => self.load_delay_timer_into(second_nibble),
                0x0A => self.wait_for_key(second_nibble),
                0x15 => self.set_delay_timer_from_reg(second_nibble),
                0x18 => self.set_sound_timer_from_reg(second_nibble),
                0x1E => self.add_to_i(second_nibble),
                0x29 => self.load_digit_into_i(second_nibble),
                0x33 => self.load_bcd_of_reg_into_i(second_nibble),
                0x55 => self.copy_registers_into_memory(second_nibble),
                0x65 => self.read_memory_into_registers(second_nibble),
                _ => panic!("Unrecognized opcode: {:X}", opcode),
            },

            _ => panic!("Unrecognized opcode: {:X}", opcode),
        }
    }

    // 00E0 - CLS
    fn clear_screen(&mut self) {
        self.display = [[0; 64]; 32];
    }

    // 00EE - RET
    fn return_from_submodule(&mut self) {
        self.pc = self.stack.pop().expect("stack was empty!");
    }

    // 1nnn - JP addr
    fn jump(&mut self, address: u16) {
        self.pc = address;
        self.pc -= 2
    }

    // 2nnn - CALL addr
    fn call(&mut self, address: u16) {
        self.stack.push(self.pc);
        self.pc = address;
        self.pc -= 2;
    }

    // 3xkk - SE Vx, byte
    // Skip next instruction if Vx = kk.
    fn skip_if_reg_equal_to_val(&mut self, register: usize, value: u8) {
        if self.registers[register] == value as u8 {
            self.pc += 2;
        }
    }

    // 4xkk - SNE Vx, byte
    // Skip next instruction if Vx != kk.
    fn skip_if_reg_not_equal_to_val(&mut self, register: usize, value: u8) {
        if self.registers[register] != value as u8 {
            self.pc += 2;
        }
    }

    // 5xy0 - SE Vx, Vy
    // Skip next instruction if Vx = Vy.
    fn skip_if_reg_equal_to_reg(&mut self, register1: usize, register2: usize) {
        if self.registers[register1] == self.registers[register2] {
            self.pc += 2;
        }
    }

    // 6xkk - LD Vx, byte
    // Set Vx = kk.
    fn load_value_into_reg(&mut self, register: usize, value: u8) {
        self.registers[register] = value;
    }

    // 7xkk - ADD Vx, byte
    // Set Vx = Vx + kk.
    fn add_value_to_reg(&mut self, register: usize, value: u8) {
        let x = self.registers[register] as u32;
        let kk = value as u32;
        if x + kk > 255 {
            self.registers[0xf] = 1;
        } else {
            self.registers[0xf] = 0;
        }

        self.registers[register] = (x + kk) as u8;
    }

    // 8xy0 - LD Vx, Vy
    // Set Vx = Vy.
    fn load_reg_into_reg(&mut self, register1: usize, register2: usize) {
        self.registers[register1] = self.registers[register2];
    }

    // 8xy1 - OR Vx, Vy
    // Set Vx = Vx OR Vy.
    fn or_reg(&mut self, register1: usize, register2: usize) {
        self.registers[register1] = self.registers[register1] | self.registers[register2];
    }

    // 8xy2 - AND Vx, Vy
    // Set Vx = Vx AND Vy.
    fn and_reg(&mut self, register1: usize, register2: usize) {
        self.registers[register1] = self.registers[register1] & self.registers[register2];
    }

    // 8xy3 - XOR Vx, Vy
    // Set Vx = Vx XOR Vy.
    fn xor_reg(&mut self, register1: usize, register2: usize) {
        self.registers[register1] = self.registers[register1] ^ self.registers[register2];
    }

    // 8xy4 - ADD Vx, Vy
    // Set Vx = Vx + Vy, set VF = carry.
    fn add_reg(&mut self, register1: usize, register2: usize) {
        let x = self.registers[register1] as u32;
        let y = self.registers[register2] as u32;
        if x + y > 255 {
            self.registers[0xf] = 1;
        } else {
            self.registers[0xf] = 0;
        }

        self.registers[register1] = (x + y) as u8;
    }

    // 8xy5 - SUB Vx, Vy
    // Set Vx = Vx - Vy, set VF = NOT borrow.
    fn sub_reg(&mut self, register1: usize, register2: usize) {
        let x = self.registers[register1];
        let y = self.registers[register2];
        if x > y {
            self.registers[0xf] = 1;
            self.registers[register1] = x - y;
        } else {
            self.registers[0xf] = 0;
            self.registers[register1] = 255 - (y - x);
        }
    }

    // 8xy6 - SHR Vx {, Vy}
    // Set Vx = Vx SHR 1.
    fn shr_reg(&mut self, register: usize) {
        println!("VF before: {:b}", self.registers[0xf]);
        self.registers[0xf] = self.registers[register] & 0x1;
        println!("VF after: {:b}", self.registers[0xf]);

        println!("register Vx before: {}", self.registers[register]);

        self.registers[register] = self.registers[register] >> 1;
        println!("register Vx after: {}", self.registers[register]);
    }

    // 8xy7 - SUBN Vx, Vy
    // Set Vx = Vy - Vx, set VF = NOT borrow.
    fn subn_reg(&mut self, register1: usize, register2: usize) {
        let x = self.registers[register1];
        let y = self.registers[register2];
        if y > x {
            self.registers[0xf] = 1;
            self.registers[register1] = y - x;
        } else {
            self.registers[0xf] = 0;
            self.registers[register1] = 255 - (x - y);
        }
    }

    // 8xyE - SHL Vx {, Vy}
    // Set Vx = Vx SHL 1.
    fn shl_reg(&mut self, register: usize) {
        self.registers[0xf] = (self.registers[register] >> 7) & 0x1;
        self.registers[register] = self.registers[register] << 1;
    }

    // 9xy0 - SNE Vx, Vy
    // Skip next instruction if Vx != Vy.
    fn skip_next_if_reg_equal_reg(&mut self, register1: usize, register2: usize) {
        if self.registers[register1] == self.registers[register2] {
            self.pc += 2;
        }
    }

    // Annn - LD I, addr
    // Set I = nnn.
    fn load_value_into_i(&mut self, value: u16) {
        self.i = value;
    }

    // Bnnn - JP V0, addr
    // Jump to location nnn + V0.
    fn jump_to_reg_zero(&mut self, address: u16) {
        self.pc = address + self.registers[0] as u16 - 2;
    }

    // Cxkk - RND Vx, byte
    // Set Vx = random byte AND kk.
    fn random(&mut self, register: usize, value: u8) {
        self.registers[register] = rand::thread_rng().gen_range(0, 255) & value;
    }

    // Dxyn - DRW Vx, Vy, nibble
    // Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
    fn draw_sprite(&mut self, xreg: usize, yreg: usize, height: u8) {
        let x = self.registers[xreg];
        let y = self.registers[yreg];

        // println!("x: {} y: {} h: {}", x, y, height);

        self.registers[0xf] = 0;

        for h in 0..height {
            let b = self.memory[self.i as usize + h as usize];
            for n in 0..8 {
                let pixel = (b >> (7 - n)) & 0x01;
                if pixel > 0 {
                    let ref mut display_pixel =
                        self.display[((y + h) % 32) as usize][((x + n) % 64) as usize];
                    self.registers[0xf] = *display_pixel | self.registers[0xf];
                    *display_pixel = *display_pixel ^ pixel;
                }
            }
        }
    }

    // Ex9E - SKP Vx
    // Skip next instruction if key with the value of Vx is pressed.
    fn skip_next_if_key_pressed(&mut self, register: usize) {
        if self.keys_pressed[self.registers[register] as usize] == true {
            self.pc += 2;
        };
    }

    // ExA1 - SKNP Vx
    // Skip next instruction if key with the value of Vx is not pressed.
    fn skip_next_if_key_not_pressed(&mut self, register: usize) {
        if self.keys_pressed[self.registers[register] as usize] == false {
            self.pc += 2;
        };
    }

    // Fx07 - LD Vx, DT
    // Set Vx = delay timer value.
    fn load_delay_timer_into(&mut self, register: usize) {
        self.registers[register] = self.dt;
    }

    // Fx0A - LD Vx, K
    // Wait for a key press, store the value of the key in Vx.
    fn wait_for_key(&mut self, register: usize) {
        for (i, x) in self.keys_pressed.into_iter().enumerate() {
            if x == &true {
                self.registers[register] = i as u8;
                return;
            }
        }

        // If we didn't find a pressed key, rewind the PC so in the next
        // cycle we hit this instruction again.
        self.pc -= 2;
    }

    // Fx15 - LD DT, Vx
    // Set delay timer = Vx.
    fn set_delay_timer_from_reg(&mut self, register: usize) {
        self.dt = self.registers[register];
    }

    // Fx18 - LD ST, Vx
    // Set sound timer = Vx.
    fn set_sound_timer_from_reg(&mut self, register: usize) {
        self.st = self.registers[register];
    }

    // Fx1E - ADD I, Vx
    // Set I = I + Vx.
    fn add_to_i(&mut self, register: usize) {
        self.i += self.registers[register] as u16;
    }

    // Fx29 - LD F, Vx
    // Set I = location of sprite for digit Vx.
    fn load_digit_into_i(&mut self, register: usize) {
        self.i = (self.registers[register] * 5) as u16;
    }

    // Fx33 - LD B, Vx
    // Store BCD representation of Vx in memory locations I, I+1, and I+2.
    fn load_bcd_of_reg_into_i(&mut self, register: usize) {
        let i = self.i as usize;
        let mut n = self.registers[register];
        let mut digit: u8;

        digit = n / 100;
        self.memory[i] = digit;
        n -= 100 * digit;

        digit = n / 10;
        self.memory[i + 1] = digit;
        n -= 10 * digit;

        self.memory[i + 2] = n;
    }

    // Fx55 - LD [I], Vx
    // Store registers V0 through Vx in memory starting at location I.
    fn copy_registers_into_memory(&mut self, max_register: usize) {
        for x in 0..=max_register {
            self.memory[self.i as usize + x] = self.registers[x];
        }
    }

    // Fx65 - LD Vx, [I]
    // Read registers V0 through Vx from memory starting at location I.
    fn read_memory_into_registers(&mut self, max_register: usize) {
        for x in 0..=max_register {
            self.registers[x] = self.memory[self.i as usize + x];
        }
    }
}

// fn draw_display(display: &[[u8; 64]; 32]) {
//     for y in 0..32 {
//         println!();
//         for x in 0..64 {
//             print!("{}", if display[y][x] != 0 { "X" } else { " " });
//         }
//     }
//     println!();
// }

fn resize_window(window: &mut sdl2::video::Window, x: u32, y: u32) {
    window.set_size(x, y).unwrap();
}

fn main() {
    // map real keypresses to what the VM expects
    let key_remapping = hashmap! {
        "1" => 0x1,
        "2" => 0x2,
        "3" => 0x3,
        "4" => 0xC,
        "Q" => 0x4,
        "W" => 0x5,
        "E" => 0x6,
        "R" => 0xD,
        "A" => 0x7,
        "S" => 0x8,
        "D" => 0x9,
        "F" => 0xE,
        "Z" => 0xA,
        "X" => 0x0,
        "C" => 0xB,
        "V" => 0xF,
    };

    let mut vm = Chip8::new();
    vm.load_application("games/SquareRootTest.ch8");

    // scale pixels by
    let scaler = 4;
    let mut current_scale = 8;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window(
            "Critter's Amazing Chip8 Emulator",
            64 * current_scale,
            32 * current_scale,
        )
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();
    let mut event_pump: sdl2::EventPump = sdl_context.event_pump().unwrap();

    let mut delay_counter = 0u32;
    let sixty_hz = (10_u32).pow(9) / 60;
    let cycle_time = (10_u32).pow(9) / 500;
    let mut t1: Instant;
    let mut t2: u32;
    let mut draw_screen = true;

    let mut paused = false;
    let mut step_instruction = false;

    // Main event loop
    'running: loop {
        // get starting time
        t1 = Instant::now();

        // get input
        for event in event_pump.poll_iter() {
            match event {
                // close button or escape shuts down the emulator
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                // PageDown makes the window smaller
                Event::KeyDown {
                    keycode: Some(Keycode::PageDown),
                    repeat: false,
                    ..
                } => {
                    if current_scale > scaler {
                        current_scale -= scaler;
                    } else {
                        current_scale = 1;
                    }
                }
                // PageUp makes the window biggger
                Event::KeyDown {
                    keycode: Some(Keycode::PageUp),
                    repeat: false,
                    ..
                } => {
                    if current_scale < scaler {
                        current_scale = scaler;
                    } else {
                        current_scale += scaler;
                    }
                    resize_window(canvas.window_mut(), 64 * current_scale, 32 * current_scale);
                }
                // Pause pauses the emulator, allowing for single-stepping instructions
                Event::KeyDown {
                    keycode: Some(Keycode::Pause),
                    repeat: false,
                    ..
                } => {
                    if paused {
                        paused = false;
                        continue 'running;
                    } else {
                        paused = true;
                        step_instruction = true;
                    };
                }
                // When paused, Period advances the emulator a single instruction
                Event::KeyDown {
                    keycode: Some(Keycode::Period),
                    repeat: false,
                    ..
                } => {
                    step_instruction = true;
                }
                _ => {}
            }
        }

        // really get input
        vm.register_keydown(
            event_pump
                .keyboard_state()
                .pressed_scancodes()
                .filter_map(|k| {
                    Some(*key_remapping.get::<str>(&Keycode::from_scancode(k)?.name())?)
                }),
        );

        // draw display
        if draw_screen {
            // clear canvas to white
            canvas.set_draw_color(Color::RGB(255, 255, 255));
            canvas.clear();
            // set color to black
            canvas.set_draw_color(Color::RGB(0, 0, 0));

            for y in 0..32 as i32 {
                for x in 0..64 as i32 {
                    if vm.display[y as usize][x as usize] == 0 {
                        canvas
                            .fill_rect(Rect::new(
                                x * current_scale as i32,
                                y * current_scale as i32,
                                current_scale,
                                current_scale,
                            ))
                            .unwrap();
                    }
                }
            }

            draw_screen = false;

            // update display
            canvas.present();
        }

        if paused {
            if step_instruction {
                vm.execute_next_instruction();
                println!("{}", vm.get_pretty_debug_info());
                step_instruction = false;
            }
        } else {
            // VM shit
            vm.execute_next_instruction();

            t2 = t1.elapsed().as_nanos() as u32;
            delay_counter += if cycle_time > t2 { cycle_time - t2 } else { 0 };
            while delay_counter > sixty_hz {
                delay_counter -= sixty_hz;
                if vm.dt > 0 {
                    vm.dt -= 1;
                };
                if vm.st > 0 {
                    vm.st -= 1;
                };
                draw_screen = true;
            }

            // println!("60hz: {}, delay_counter: {}", sixty_hz, delay_counter);
            // println!("cycle_time: {}, t2: {}", cycle_time, t2);

            if cycle_time > t2 {
                sleep(Duration::new(0, cycle_time - t2));
            }
        }
    }
}
