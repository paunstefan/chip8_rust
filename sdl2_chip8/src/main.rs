use core::time;
use std::error::Error;

use ::chip8::*;
use rand::Rng;
use sdl2::{
    event::Event,
    keyboard::Keycode,
    pixels::{Color, PixelFormatEnum},
};

const SCALE: usize = 20;

fn main() {
    run().unwrap();
}

fn run() -> Result<(), Box<dyn Error>> {
    let sdl_context = sdl2::init()?;
    let video = sdl_context.video()?;

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
                _ => {}
            }
        }

        canvas.clear();
        canvas.copy(&tex_display, None, None)?;
        canvas.present();

        std::thread::sleep(time::Duration::from_millis(16));
    }

    Ok(())
}
