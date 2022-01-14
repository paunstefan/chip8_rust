use core::time;
use std::{env, error::Error, fs::File, io::Read};

use ::chip8::io::Random;
use ::chip8::*;
use rand::prelude::*;
use rand::Rng;
use sdl2::audio::AudioCallback;
use sdl2::audio::AudioSpecDesired;
use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum};

const SCALE: usize = 20;

struct RandomNum {
    rng: ThreadRng,
}

impl RandomNum {
    fn new() -> Self {
        Self {
            rng: rand::thread_rng(),
        }
    }
}

impl Random for RandomNum {
    fn randint(&mut self) -> u8 {
        self.rng.gen()
    }
}

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

#[allow(non_snake_case)]
fn print_debug_info(machine: &chip8::Chip8<RandomNum>) {
    let (PC, instruction, V, I) = machine.get_debug_info();
    println!(
        "{:x} {:x} {}",
        PC,
        instruction,
        chip8::Chip8::<RandomNum>::print_instruction(instruction)
    );
    for r in V.iter().take(15) {
        print!("{} ", r);
    }
    println!("I: {}", I);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: chip8_sdl2 [rom_file]");
        std::process::exit(1);
    }
    let mut file = File::open(&args[1]).unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    let random = RandomNum::new();

    let chip8 = chip8::Chip8::new(&data, random);

    run(chip8).unwrap();
}

fn run(mut machine: chip8::Chip8<RandomNum>) -> Result<(), Box<dyn Error>> {
    let sdl_context = sdl2::init()?;
    let video = sdl_context.video()?;
    let audio = sdl_context.audio()?;

    // Initialize audio device
    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1), // mono
        samples: None,     // default sample size
    };

    let device = audio.open_playback(None, &desired_spec, |spec| {
        // initialize the audio callback
        SquareWave {
            phase_inc: 440.0 / spec.freq as f32,
            phase: 0.0,
            volume: 0.25,
        }
    })?;

    // Initialize video device
    let window = video
        .window(
            "chip8-sdl2",
            (chip8::SCREEN_WIDTH * SCALE) as u32,
            (chip8::SCREEN_HEIGHT * SCALE) as u32,
        )
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();
    let mut tex_display = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGB24,
            chip8::SCREEN_WIDTH as u32,
            chip8::SCREEN_HEIGHT as u32,
        )
        .map_err(|e| e.to_string())?;

    let mut event_pump = sdl_context.event_pump()?;

    'gameloop: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'gameloop,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    let index = match key {
                        Keycode::Kp7 => 0,
                        Keycode::Kp8 => 1,
                        Keycode::Kp9 => 2,
                        Keycode::Kp4 => 3,
                        Keycode::Kp5 => 4,
                        Keycode::Kp6 => 5,
                        Keycode::Kp1 => 6,
                        Keycode::Kp2 => 7,
                        Keycode::Kp3 => 8,
                        Keycode::Q => 9,
                        Keycode::W => 10,
                        Keycode::E => 11,
                        Keycode::R => 12,
                        Keycode::A => 13,
                        Keycode::S => 14,
                        Keycode::D => 15,
                        _ => 16,
                    };
                    machine.set_key(index, true);
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    let index = match key {
                        Keycode::Kp7 => 0,
                        Keycode::Kp8 => 1,
                        Keycode::Kp9 => 2,
                        Keycode::Kp4 => 3,
                        Keycode::Kp5 => 4,
                        Keycode::Kp6 => 5,
                        Keycode::Kp1 => 6,
                        Keycode::Kp2 => 7,
                        Keycode::Kp3 => 8,
                        Keycode::Q => 9,
                        Keycode::W => 10,
                        Keycode::E => 11,
                        Keycode::R => 12,
                        Keycode::A => 13,
                        Keycode::S => 14,
                        Keycode::D => 15,
                        _ => 16,
                    };
                    machine.set_key(index, false)
                }
                _ => {}
            }
        }

        print_debug_info(&machine);

        for _ in 0..10 {
            machine.execute_instruction();
        }

        machine.decrement_delay();

        if machine.sound_tick() {
            device.resume();
        } else {
            device.pause()
        }

        tex_display.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
            for y in 0..chip8::SCREEN_HEIGHT {
                for x in 0..chip8::SCREEN_WIDTH {
                    let pixel = machine.get_pixel(x, y);

                    let color: u8 = if pixel { 255 } else { 0 };
                    let pos = (y * chip8::SCREEN_WIDTH + x) * 3;

                    buffer[pos] = color;
                    buffer[pos + 1] = color;
                    buffer[pos + 2] = color;
                }
            }
        })?;

        canvas.clear();
        canvas.copy(&tex_display, None, None)?;
        canvas.present();

        std::thread::sleep(time::Duration::from_millis(15));
    }

    Ok(())
}
