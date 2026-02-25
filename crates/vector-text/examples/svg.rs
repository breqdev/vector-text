use svg::Document;
use svg::node::element::Path;
use svg::node::element::path::Data;

use vector_text::{BorlandFont, HersheyFont, VectorFont, render_text};

fn points_to_svg_path(
    points: &[vector_text_core::Point],
    scale: f32,
    margin: f32,
    y_offset: f32,
) -> (Data, (f32, f32)) {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for p in points {
        min_x = min_x.min(p.x as f32);
        min_y = min_y.min(p.y as f32);
        max_x = max_x.max(p.x as f32);
        max_y = max_y.max(p.y as f32);
    }

    let width = (max_x - min_x) * scale + 2.0 * margin;
    let height = (max_y - min_y) * scale + 2.0 * margin;

    let mut data = Data::new();

    let mut pen_up = true;

    for p in points {
        let x = (p.x as f32 - min_x) * scale + margin;
        let y = (p.y as f32 - min_y) * scale + margin;

        if !p.pen {
            data = data.move_to((x, y + y_offset));
            pen_up = false;
        } else {
            if pen_up {
                data = data.move_to((x, y + y_offset));
                pen_up = false;
            } else {
                data = data.line_to((x, y + y_offset));
            }
        }
    }

    (data, (width, height))
}

fn draw_font_line(
    text: &str,
    font: VectorFont,
    y_offset: f32,
    scale: f32,
    margin: f32,
    line_height: f32,
) -> (Path, f32) {
    let points = render_text(text, font);

    let (data, _) = points_to_svg_path(&points, scale, margin, y_offset);

    let path = Path::new()
        .set("fill", "none")
        .set("stroke", "black")
        .set("stroke-width", 1)
        .set("d", data);

    (path, y_offset + line_height)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use svg::node::element::{Path, Rectangle};

    let scale = 1.0;
    let margin = 10.0;
    let line_height = 40.0;

    let mut y_offset = 0.0;
    let mut elements: Vec<Path> = Vec::new();

    let (p, y) = draw_font_line(
        "Hershey Roman Simplex",
        VectorFont::HersheyFont(HersheyFont::Romans),
        y_offset,
        scale,
        margin,
        line_height,
    );
    elements.push(p);
    y_offset = y;

    let (p, y) = draw_font_line(
        "Hershey Gothic English",
        VectorFont::HersheyFont(HersheyFont::Gotheng),
        y_offset,
        scale,
        margin,
        line_height,
    );
    elements.push(p);
    y_offset = y;

    let (p, y) = draw_font_line(
        "Hershey Italic Complex",
        VectorFont::HersheyFont(HersheyFont::Italicc),
        y_offset,
        scale,
        margin,
        line_height,
    );
    elements.push(p);
    y_offset = y;

    let (p, y) = draw_font_line(
        "Hershey Roman Triplex",
        VectorFont::HersheyFont(HersheyFont::Romant),
        y_offset,
        scale,
        margin,
        line_height,
    );
    elements.push(p);
    y_offset = y;

    let (p, y) = draw_font_line(
        "Borland LITT.CHR",
        VectorFont::BorlandFont(BorlandFont::Litt),
        y_offset,
        scale,
        margin,
        line_height,
    );
    elements.push(p);
    y_offset = y;

    let (p, y) = draw_font_line(
        "Borland EURO.CHR",
        VectorFont::BorlandFont(BorlandFont::Euro),
        y_offset - 16.0,
        scale,
        margin,
        line_height,
    );
    elements.push(p);
    y_offset = y;

    let (p, y) = draw_font_line(
        "Borland GOTH.CHR",
        VectorFont::BorlandFont(BorlandFont::Goth),
        y_offset + 32.0,
        scale,
        margin,
        line_height,
    );
    elements.push(p);
    y_offset = y;

    let (p, y) = draw_font_line(
        "NewStroke (KiCAD Font)",
        VectorFont::NewstrokeFont(()),
        y_offset,
        scale,
        margin,
        line_height,
    );
    elements.push(p);
    y_offset = y;

    let height = y_offset + margin;
    let width = 500.0;

    let background = Rectangle::new()
        .set("x", 0)
        .set("y", 0)
        .set("width", width)
        .set("height", height)
        .set("fill", "white");

    let mut document = Document::new().add(background);

    for el in elements {
        document = document.add(el);
    }

    let document = document
        .set("viewBox", (0, 0, width, height))
        .set("width", format!("{width}px"))
        .set("height", format!("{height}px"));

    svg::save("output_fonts.svg", &document)?;
    println!("Wrote output_fonts.svg");

    Ok(())
}
