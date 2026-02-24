#![no_std]

//! `vector-text-borland` is a backend for the `vector-text` crate that
//! renders fonts in the BGI (Borland Graphics Interface) format.
//!
//! It includes the standard BGI fonts from GameMaker, which are available
//! under an MIT license. For more details, see the original source here:
//! <https://github.com/gandrewstone/GameMaker>

extern crate alloc;

use alloc::vec::Vec;

use vector_text_core::{Glyph, PackedPoint, Point, Renderer};

include!(concat!(env!("OUT_DIR"), "/chr_font.rs"));

/// A [Renderer] which draws text using Borland fonts.
pub struct BorlandRenderer;

impl Renderer<BorlandFont> for BorlandRenderer {
    fn render_text(text: &str, font: BorlandFont) -> Vec<Point> {
        let mut result = Vec::new();
        let mut x_idx = 0;

        let table = font.table();

        for character in text.chars() {
            if let Some(glyph) = table[character as usize] {
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
