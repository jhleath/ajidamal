#![deny(warnings)]
#![allow(dead_code)]

extern crate framebuffer;
extern crate rusttype;
extern crate chrono;
extern crate ajidamal;

use ajidamal::display::{Screen};
use ajidamal::display::base::{Color};

use rusttype::{FontCollection, Scale, point, PositionedGlyph};
use chrono::prelude::*;

fn main() {
    let mut screen = Screen::new("/dev/fb1".to_string());

    // Save the width and height to variables so that we don't borrow screen.
    let (screen_width, screen_height) = screen.dimensions();

    // This font is not included in the source code of this
    // repository, you should be able to Google and find it.
    let font_data = include_bytes!("../../fonts/source_code_pro.ttf");

    let collection = FontCollection::from_bytes(font_data as &[u8]);
    let font = collection.into_font().unwrap(); // only succeeds if collection consists of one font

    // Desired font pixel height
    let height: f32 = 14.0; // to get 80 chars across (fits most terminals); adjust as desired
    let pixel_height = height.ceil() as usize;

    // 2x scale in x direction to counter the aspect ratio of monospace characters.
    let scale = Scale { x: height, y: height };

    // The origin of a line of text is at the baseline (roughly where non-descending letters sit).
    // We don't want to clip the text, so we shift it down with an offset when laying it out.
    // v_metrics.ascent is the distance between the baseline and the highest edge of any glyph in
    // the font. That's enough to guarantee that there's no clipping.
    let v_metrics = font.v_metrics(scale);
    let offset = point(3.0, v_metrics.ascent);

    let local: DateTime<Local> = Local::now();

    // Glyphs to draw for "RustType". Feel free to try other strings.
    let glyphs: Vec<PositionedGlyph> = font.layout(
        &format!("{}", local.format("%H:%M %p")), scale, offset).collect();

    // Find the most visually pleasing width to display
    let text_width = glyphs.iter().rev()
        .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
        .next().unwrap_or(0.0).ceil() as usize;

    println!("width: {}, height: {}", text_width, pixel_height);

    let text_start_x = (screen_width - text_width as u32) - 5;

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

    for g in glyphs {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|x, y, v| {
                // v should be in the range 0.0 to 1.0
                let color = Color::gray((255_f32 * v).round() as u64);

                let x = x as i32 + bb.min.x;
                let y = y as i32 + bb.min.y;
                // There's still a possibility that the glyph clips the boundaries of the bitmap
                if x >= 0 && x < text_width as i32 && y >= 0 && y < pixel_height as i32 {
                    let x = x as usize;
                    let y = y as usize;

                    screen.write_pixel(x + (text_start_x as usize), y + 1, color);
                }
            })
        }

        screen.flush()
    }
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
