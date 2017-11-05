#![deny(warnings)]
#![allow(dead_code)]

extern crate rusttype;
extern crate chrono;
extern crate ajidamal;

use ajidamal::display::{Screen};
use ajidamal::display::base::{Color, Point};
use ajidamal::display::view::{View};
use ajidamal::display::text::{TextRenderer};

use chrono::prelude::*;

fn main() {
    let mut screen = Screen::new("/dev/fb1".to_string());
    let text_renderer = TextRenderer::new();

    // Save the width and height to variables so that we don't borrow screen.
    let (screen_width, screen_height) = screen.dimensions();

    let local: DateTime<Local> = Local::now();
    let time_view = text_renderer.rasterize(/*size=*/14.0, Color::gray(/*intensity=*/255),
                                            format!("{}", local.format("%H:%M %p")));

    let text_start_x = (screen_width - time_view.width() as u32) - 5;

    screen.with_root_view(move |rv: &mut View| {
        rv.add_full_subview(time_view, Point::new(text_start_x as u64, 1))
    });

    // Split the display into a status bar and a main area
    let status_bar_height = 17;
    draw_rectangle(&mut screen, 0, 0,
                   screen_width, status_bar_height,
                   Color::gray(0));
    draw_rectangle(&mut screen, 0, status_bar_height,
                   screen_width, screen_height - status_bar_height,
                   Color::gray(255));

    let box_height = 50;
    let box_color = Color::new(17, 132, 255);

    draw_rectangle(&mut screen, 2, status_bar_height + 2,
                   screen_width - 4, box_height, box_color);
    draw_rectangle(&mut screen, 2, status_bar_height + 2 * 2 + box_height,
                   screen_width - 4, box_height, box_color);
    draw_rectangle(&mut screen, 2, status_bar_height + 2 * 3 + box_height * 2,
                   screen_width - 4, screen_height - (status_bar_height + 2 * 3 + box_height * 2),
                   box_color);

    screen.render_view();
}

fn draw_rectangle(screen: &mut Screen, x: u32, y: u32, w: u32, h: u32, color: Color) {
    // TODO: [2017-11-01] hleath: Implement a blit operation to
    // mass-copy bytes into the mmap'd frame buffer. It is super
    // inefficient to loop about like this.

    let mut horiz = x as usize;
    while horiz < (x + w) as usize {
        let mut vert = y as usize;

        while vert < (y + h) as usize {
            screen.write_pixel(horiz, vert, color);

            vert += 1;
        }

        horiz += 1;
    }

    screen.flush()
}
