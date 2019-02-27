use cairo::{RectangleInt, Region};
use engiffen::{engiffen, Image, Quantizer};
use gtk::prelude::*;
use notify_rust::Notification;
use scrap::{Capturer, Display};
use std::{
    fs::File,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

pub static RECORDING: AtomicBool = AtomicBool::new(false);

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
        },
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
    let display = Display::primary().expect("Couldn't find primary display.");
    let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");
    let (screen_w, screen_h) = (capturer.width(), capturer.height());
    let mut images = Vec::new();
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
        if RECORDING.load(Ordering::Relaxed) {
            break;
        }
    }
    let gif = engiffen(&images, data.fps, data.quantizer).unwrap();
    let mut output = File::create(&data.path).unwrap();
    gif.write(&mut output).unwrap();
    RECORDING.compare_and_swap(true, false, Ordering::Relaxed);
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
    let builder = Arc::new(gtk::Builder::new_from_string(glade_src));
    let window: Arc<gtk::Window> = Arc::new(builder.get_object("main_window").unwrap());
    window.set_keep_above(true);
    window.connect_draw(draw);
    window.set_size_request(500, 500);
    set_visual(&window, &None);
    window.set_app_paintable(true);
    let button: gtk::Button = builder.get_object("record_button").unwrap();
    let builder_clone = builder.clone();
    window.connect_draw(move |window, _| {
        let window_region =
            get_widget_region::<gtk::Window>("main_window", &builder_clone);
        let view_region = get_widget_region::<gtk::Box>("main_box", &builder_clone);
        let header_region =
            get_widget_region::<gtk::HeaderBar>("main_header", &builder_clone);

        window_region.subtract(&view_region);
        window_region.union(&header_region);
        window.input_shape_combine_region(&window_region);
        Inhibit(false)
    });
    let window_in_button = window.clone();
    button.connect_clicked(move |a| {
        if a.get_label().unwrap() == "Stop" {
            RECORDING.compare_and_swap(false, true, Ordering::Relaxed);
            a.set_label("Record");
        } else {
            a.set_label("Stop");
            let (x, y) = window_in_button.get_position();
            let (w, h) = window_in_button.get_size();
            let header_region =
                get_widget_region::<gtk::HeaderBar>("main_header", &builder);
            let header_rectangle = header_region.get_rectangle(0);
            std::thread::spawn(move || {
                let data = Data {
                    path: "output.gif".to_string(),
                    fps: 10,
                    quantizer: Quantizer::Naive,
                };
                start_recording(
                    x as usize,
                    y as usize + header_rectangle.height as usize + 25, //TODO: remove hardcoded coordinates
                    w as usize,
                    h as usize - header_rectangle.height as usize,
                    &data,
                );
                Notification::new()
                    .summary("Magnus Gif saved")
                    .icon("dialog-information")
                    .show()
                    .unwrap();
            });
        }
    });

    window.show_all();

    gtk::main();
}
