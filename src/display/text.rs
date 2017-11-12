extern crate rusttype;

use self::rusttype::{Font, FontCollection, Scale, point, PositionedGlyph};

use super::base::{Color};
use super::view::{Buffer};

// TODO: [hleath 2017-11-02] Don't include resources in the codebase
// like this. Of course, that will require a real resource loader for
// this program, and that seems pretty far off.
//
// This font is not included in the source code of this
// repository, you should be able to Google and find it.
static DEFAULT_FONT_DATA: &[u8] = include_bytes!("../../fonts/source_code_pro.ttf");

pub struct TextRenderer<'a> {
    font: Font<'a>,
}

impl<'a> TextRenderer<'a> {
    pub fn new() -> TextRenderer<'a> {
        let collection = FontCollection::from_bytes(DEFAULT_FONT_DATA as &[u8]);
        let font = collection.into_font().unwrap(); // only succeeds if collection consists of one font

        TextRenderer {
            font: font,
        }
    }

    pub fn rasterize(&self, size: f32, color: Color, data: &str) -> Buffer {
        let pixel_height = size.ceil() as usize;

        // Uniformly scale the font to the requested size.
        let scale = Scale {
            x: size,
            y: size,
        };

        // The origin of a line of text is at the baseline (roughly where non-descending letters sit).
        // We don't want to clip the text, so we shift it down with an offset when laying it out.
        // v_metrics.ascent is the distance between the baseline and the highest edge of any glyph in
        // the font. That's enough to guarantee that there's no clipping.
        let v_metrics = self.font.v_metrics(scale);
        let offset = point(0.0, v_metrics.ascent);

        // Draw the actual glyphs
        let glyphs: Vec<PositionedGlyph> = self.font.layout(data, scale, offset).collect();

        // Find the most visually pleasing width to display
        let text_width = glyphs.iter().rev()
            .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
            .next().unwrap_or(0.0).ceil() as usize;

        let mut text_buffer = Buffer::new(text_width as u64, pixel_height as u64);

        for g in glyphs {
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw(|x, y, v| {
                    // v should be in the range 0.0 to 1.0
                    let pixel_color = color.with_opacity(v);

                    let x = x as i32 + bb.min.x;
                    let y = y as i32 + bb.min.y;
                    // There's still a possibility that the glyph clips the boundaries of the bitmap
                    if x >= 0 && x < text_width as i32 && y >= 0 && y < pixel_height as i32 {
                        let x = x as usize;
                        let y = y as usize;

                        text_buffer.write_pixel(x, y, pixel_color);
                    }
                })
            }
        }

        text_buffer
    }
}
