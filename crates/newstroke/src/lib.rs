#![no_std]

//! `vector-text-newstroke` is a backend for the `vector-text` crate that
//! renders the NewStroke font (originally created for KiCAD).
//!
//! Data for the NewStroke font was sourced from the project page: <https://vovanium.ru/sledy/newstroke/en>

extern crate alloc;

use alloc::vec::Vec;
use vector_text_core::{Glyph, PackedPoint, Point, Renderer};

include!(concat!(env!("OUT_DIR"), "/newstroke_font.rs"));

/// A [Renderer] which draws text using the NewStroke font.
pub struct NewstrokeRenderer;

impl Renderer<()> for NewstrokeRenderer {
    fn render_text(text: &str, _mapping: ()) -> Vec<Point> {
        let mut result = Vec::new();
        let mut x_idx = 0;

        for character in text.chars() {
            if let Some(glyph) = NEWSTROKE_FONT[character as usize] {
                result.extend(glyph.strokes.iter().map(|point| Point {
                    x: point.x as i16 - glyph.left as i16 + x_idx,
                    y: point.y as i16,
                    pen: point.pen,
                }));
                x_idx += glyph.right as i16 - glyph.left as i16;
            }
        }

        result
    }
}
