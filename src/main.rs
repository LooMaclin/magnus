#![feature(duration_float)]

use cairo::RectangleInt;
use cairo::Region;
use engiffen::engiffen;
use engiffen::Image;
use engiffen::Quantizer;
use gdk::Window;
use gdk::WindowExt;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::Align;
use gtk::Orientation;
use gtk::{ApplicationWindow, Button, Fixed};
use scrap::{Capturer, Display};
use std::env::args;
use std::fs::File;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;

fn flip_flop(
    buffer: &[u8],
    w: usize,
    h: usize,
    start_x: usize,
    start_y: usize,
    image_w: usize,
    image_h: usize,
) -> Vec<[u8; 4]> {
    let mut bitflipped = Vec::with_capacity(w * h * 4);
    let stride = buffer.len() / h;

    for y in start_y..start_y + image_h {
        for x in start_x..start_x + image_w {
            let i = stride * y + 4 * x;
            bitflipped.push([buffer[i + 2], buffer[i + 1], buffer[i], 255]);
        }
    }

    bitflipped
}

fn capture_screen(
    capturer: &mut Capturer,
    screen_w: usize,
    screen_h: usize,
    x: usize,
    y: usize,
    image_w: usize,
    image_h: usize,
) -> Image {
    let buffer = match capturer.frame() {
        Ok(buffer) => buffer,
        Err(error) => {
            panic!("Error: {:#?}", error);
        }
    };
    let image = flip_flop(&buffer, screen_w, screen_h, x, y, image_w, image_h);
    let image = Image {
        pixels: image,
        width: image_w as u32,
        height: image_h as u32,
    };
    image
}

struct Data {
    pub path: String,
    pub fps: usize,
    pub quantizer: Quantizer,
}

fn start_recording(x: usize, y: usize, w: usize, h: usize, data: &Data) {
    println!("start recording");

    let display = Display::primary().expect("Couldn't find primary display.");
    let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");
    let (screen_w, screen_h) = (capturer.width(), capturer.height());
    let mut images = Vec::new();
    let instant = Instant::now();
    loop {
        images.push(capture_screen(
            &mut capturer,
            screen_w,
            screen_h,
            x,
            y,
            w,
            h,
        ));
        std::thread::sleep(Duration::from_millis(16));
        if images.len() == 640 {
            break;
        }
    }
    println!("elapsed: {}", instant.elapsed().as_float_secs());
    let gif = engiffen(&images, data.fps, data.quantizer).unwrap();
    let mut output = File::create(&data.path).unwrap();
    gif.write(&mut output).unwrap();
    println!("saved to {}", data.path);
}

fn set_visual(window: &gtk::Window, _screen: &Option<gdk::Screen>) {
    if let Some(screen) = window.get_screen() {
        if let Some(visual) = screen.get_rgba_visual() {
            window.set_visual(&visual);
        }
    }
}

fn draw(_window: &gtk::Window, ctx: &cairo::Context) -> Inhibit {
    ctx.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    ctx.set_operator(cairo::Operator::Screen);
    ctx.paint();
    Inhibit(false)
}

fn get_widget_region<T: IsA<gtk::Object> + IsA<gtk::Widget>>(
    name: &'static str,
    builder: &gtk::Builder,
) -> Region {
    let widget: T = builder.get_object(name).unwrap();
    let width = widget.get_allocated_width();
    let height = widget.get_allocated_height();
    let (x, y) = widget.translate_coordinates(&widget, 0, 0).unwrap();
    let rectangle = RectangleInt {
        x,
        y,
        width,
        height,
    };

    Region::create_rectangle(&rectangle)
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }
    let glade_src = include_str!("../ui.glade");
    let builder = gtk::Builder::new_from_string(glade_src);
    let window: Arc<gtk::Window> = Arc::new(builder.get_object("main_window").unwrap());
    window.set_keep_above(true);
    window.connect_draw(draw);
    window.set_size_request(500, 500);
    set_visual(&window, &None);
    window.set_app_paintable(true);
    let button: gtk::Button = builder.get_object("record_button").unwrap();
    window.connect_draw(move |a, b| {
        let window_region = get_widget_region::<gtk::Window>("main_window", &builder);
        let view_region = get_widget_region::<gtk::Box>("main_box", &builder);
        let header_region = get_widget_region::<gtk::HeaderBar>("main_header", &builder);

        window_region.subtract(&view_region);
        window_region.union(&header_region);
        a.input_shape_combine_region(&window_region);
        Inhibit(false)
    });
    let window_in_button = window.clone();
    button.connect_clicked(move |a| {
        //        let window: Window = a.get_window().unwrap();
        let (x, y) = window_in_button.get_position();
        let (w, h) = window_in_button.get_size();
        start_recording(
            x as usize,
            y as usize,
            w as usize,
            h as usize,
            &Data {
                path: "output.gif".to_string(),
                fps: 64,
                quantizer: Quantizer::Naive,
            },
        );
    });

    window.show_all();

    gtk::main();
}
