#![no_std]

//! `vector-text-core` provides core primitives for the `vector-text` crate.

use alloc::vec::Vec;

extern crate alloc;

/// A point, in compact representation.
/// Used to store the points which make up an individual glyph.
#[derive(Debug, Copy, Clone)]
pub struct PackedPoint {
    /// X coordinate of this point
    pub x: i8,
    /// Y coordinate of this point
    pub y: i8,
    /// Should a line be drawn (i.e., "pen down") when moving to this point?
    pub pen: bool,
}

/// A single glyph (character) contained within a font.
#[derive(Debug, Copy, Clone)]
pub struct Glyph {
    /// Left coordinate boundary of this glyph
    pub left: i8,
    /// Right coordinate boundary of this glyph
    pub right: i8,
    /// Series of points which make up this glyph
    pub strokes: &'static [PackedPoint],
}

/// Representation of a point with higher range than [PackedPoint].
/// Used for the output of text rendering.
pub struct Point {
    pub x: i16,
    pub y: i16,
    pub pen: bool,
}

impl Default for Point {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            pen: false,
        }
    }
}

/// Allows rendering text into vector points.
///
/// Implementors may define their own font mapping (enum or other data structure).
pub trait Renderer<Mapping> {
    /// Render the given text string to a series of points,
    /// using the given font mapping.
    fn render_text(text: &str, mapping: Mapping) -> Vec<Point>;
}
