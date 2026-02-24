#![no_std]

//! `vector-text-hershey` is a backend for the `vector-text` crate that
//! renders Hershey fonts.
//!
//! It includes Hershey font data sourced from [Paul Bourke's compilation](https://paulbourke.net/dataformats/hershey/).

extern crate alloc;

use alloc::vec::Vec;
use vector_text_core::{Glyph, PackedPoint, Point, Renderer};

include!(concat!(env!("OUT_DIR"), "/hershey_font.rs"));

/// A [Renderer] which draws text using Hershey fonts.
pub struct HersheyRenderer;

impl Renderer<HersheyFont> for HersheyRenderer {
    fn render_text(text: &str, font: HersheyFont) -> Vec<Point> {
        let mut result = Vec::new();
        let mut x_idx = 0;

        let mapping = font.table();

        for character in text.chars() {
            if character > 255 as char {
                continue;
            }

            let hershey_id = mapping[character as usize] as usize;

            if hershey_id == 0 || hershey_id >= HERSHEY_FONT.len() {
                continue;
            }

            if let Some(glyph) = HERSHEY_FONT[hershey_id] {
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
