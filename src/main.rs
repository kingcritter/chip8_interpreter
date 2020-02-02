extern crate sdl2;
#[macro_use]
extern crate maplit;

mod chip8;

use chip8::Chip8;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

use std::env;
use std::time::Instant;

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

    // Assume the first argument is a path to to an application
    match env::args().nth(1) {
        Some(path) => vm.load_application(&path),
        None => panic!("No program specified!"),
    }

    // scale pixels by
    let scaler = 4;
    let mut current_scale = 8;

    // set up the SDL window
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

    // this is used to get keyboard input
    let mut event_pump: sdl2::EventPump = sdl_context.event_pump().unwrap();

    // variables needed for calculating the delay to emulate a slower clock speed
    let mut elapsed = 0u32;
    let mut elapsed_cpu = 0u32;
    let mut elapsed_display = 0u32;
    let sixty_hz = (10_u32).pow(9) / 60;
    let fivehundred_hertz = (10_u32).pow(9) / 500;
    let display_speed = (10_u32).pow(9) / 23;
    let mut current = Instant::now();

    // variables for pausing and single-stepping instructions
    let mut paused = false;
    let mut step_instruction = false;

    // Main event loop
    'running: loop {
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

        // send keypresses to chip8
        vm.register_keydown(
            event_pump
                .keyboard_state()
                .pressed_scancodes()
                .filter_map(|k| {
                    Some(*key_remapping.get::<str>(&Keycode::from_scancode(k)?.name())?)
                }),
        );

        let t = current.elapsed().as_nanos() as u32;
        current = Instant::now();
        if paused {
            if step_instruction {
                if vm.dt > 0 {
                    vm.dt -= 1;
                };
                if vm.st > 0 {
                    vm.st -= 1;
                };
                vm.execute_next_instruction();
                println!("{}", vm.get_pretty_debug_info());
                step_instruction = false;
            }
        } else {
            elapsed += t;
            elapsed_cpu += t;

            // the timer needs to decrement at 60 hz, realtime
            while elapsed > sixty_hz {
                elapsed -= sixty_hz;
                if vm.dt > 0 {
                    vm.dt -= 1;
                };
                if vm.st > 0 {
                    vm.st -= 1;
                };
            }
            // certain games require a 500hz clock speed
            while elapsed_cpu > fivehundred_hertz {
                elapsed_cpu -= fivehundred_hertz;
                vm.execute_next_instruction();
            }
        }

        // draw display
        elapsed_display += t;
        while vm.display_changed || elapsed_display > display_speed {
            elapsed_display = 0;
            vm.display_changed = false;

            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas.clear();
            canvas.set_draw_color(Color::RGB(255, 255, 255));
            for y in 0..32 as i32 {
                for x in 0..64 as i32 {
                    if vm.display()[y as usize][x as usize] != 0 {
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
            canvas.present();
            break;
        }
    }
}
