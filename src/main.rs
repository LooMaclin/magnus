#![feature(duration_float)]

use scrap::{Capturer, Display};
use std::io::ErrorKind::WouldBlock;
use std::fs::File;
use std::thread;
use std::time::Duration;
use engiffen::Image;
use engiffen::Quantizer;
use engiffen::engiffen;
use std::time::Instant;
use azul::prelude::*;
use azul::widgets::label::Label;
use azul::widgets::button::Button;
use std::sync::Arc;
use std::sync::Mutex;

fn flip_flop(buffer: &[u8], w: usize, h: usize) -> Vec<[u8; 4]> {
    let mut bitflipped = Vec::with_capacity(w * h * 4);
    let stride = buffer.len() / h;

    for y in 0..h {
        for x in 0..w {
            let i = stride * y + 4 * x;
            bitflipped.push([
                buffer[i + 2],
                buffer[i + 1],
                buffer[i],
                255,
            ]);
        }
    }

    bitflipped
}

fn capture_screen(capturer: &mut Capturer, w: usize, h: usize) -> Image {
    let buffer = match capturer.frame() {
        Ok(buffer) => buffer,
        Err(error) => {
            panic!("Error: {:#?}", error);
        }
    };
    let image = flip_flop(&buffer, w, h);
    let image = Image {
        pixels: image,
        width: w as u32,
        height: h as u32,
    };
    image
}


struct Data {
    pub path: String,
    pub fps: usize,
    pub quantizer: Quantizer,
}

impl Layout for Data {
    fn layout(&self, _info: LayoutInfo<Self>) -> Dom<Self> where Self: Sized {
        let label = Label::new(format!("{}", self.path)).dom();
        let button = Button::with_label("Update counter").dom()
            .with_callback(On::MouseUp, Callback(|a, b| {
                a.add_task(Task::new(&a.data,start_recording));
                Redraw
            }));

        Dom::div()
            .with_child(label)
            .with_child(button)
    }
}

fn start_recording(app_data: Arc<Mutex<Data>>, _: Arc<()>) {
    println!("start recording");
    let data = app_data.try_lock().unwrap();

    let display = Display::primary().expect("Couldn't find primary display.");
    let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");
    let (w, h) = (capturer.width(), capturer.height());
    let mut images = Vec::new();
    let instant = Instant::now();
    loop {
        images.push(capture_screen(&mut capturer, w, h));
        std::thread::sleep(Duration::from_millis(16));
        if images.len() == 640 {
            break;
        }
    }
    println!("elapsed: {}", instant.elapsed().as_float_secs());
    let gif = engiffen(&images, data.fps, data.quantizer).unwrap();
    let mut output = File::create(&data.path).unwrap();
    gif.write(&mut output).unwrap();
}

fn main() {
    let app = App::new(Data { path: "output.gif".to_string(), fps: 64, quantizer: Quantizer::Naive }, AppConfig::default());
    app.run(Window::new(WindowCreateOptions::default(), css::native()).unwrap()).unwrap();
}