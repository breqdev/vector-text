use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Debug, Copy, Clone)]
struct PackedPoint {
    pub x: i8,
    pub y: i8,
    pub pen: bool,
}

const NUM_GLYPHS: usize = 0x27FF;
type FontFile = [Option<Glyph>; NUM_GLYPHS];

/// Generate the Rust code defining the glyph table for this font.
fn generate_rust(font: &[Option<Glyph>]) -> String {
    let mut out = String::new();

    // Write the symbol table
    out.push_str(&format!(
        "static NEWSTROKE_FONT: [Option<Glyph>; {}] = [\n",
        font.len()
    ));

    for glyph in font {
        match glyph {
            None => out.push_str("    None,\n"),
            Some(g) => {
                out.push_str("    Some(Glyph {\n");
                out.push_str(&format!("        left: {},\n", g.left));
                out.push_str(&format!("        right: {},\n", g.right));
                out.push_str("        strokes: &[\n");

                for p in &g.strokes {
                    out.push_str(&format!(
                        "            PackedPoint {{ x: {}, y: {}, pen: {} }},\n",
                        p.x, p.y, p.pen
                    ));
                }

                out.push_str("        ],\n    }),\n");
            }
        }
    }

    out.push_str("];\n");

    out
}

#[derive(Debug, Clone)]
struct Glyph {
    pub left: i8,
    pub right: i8,
    pub strokes: Vec<PackedPoint>,
}

#[derive(Debug, Clone)]
struct Symbol {
    pub name: String,

    /// Vector strokes: strokes → points
    pub strokes: Vec<Vec<(i8, i8)>>,

    /// Left side bearing
    pub left: i8,

    /// Right side bearing
    pub right: i8,

    /// Anchor points (e.g. ABOVE, BELOW, MIDBOTTOM, etc)
    pub anchors: HashMap<String, (i8, i8)>,
}

const SCALE: i32 = 50;

fn conv_x(x: i32) -> i8 {
    (x / SCALE).clamp(-128, 127) as i8
}

fn conv_y(y: i32) -> i8 {
    (-y / SCALE).clamp(-128, 127) as i8
}

/// Load the .lib file defining NewStroke font symbols.
fn parse_lib_file(input: &str) -> Result<HashMap<String, Symbol>, String> {
    let mut glyphs = HashMap::new();

    let mut current: Option<Symbol> = None;

    for (lineno, line) in input.lines().enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "DEF" => {
                if current.is_some() {
                    return Err(format!("Nested DEF at line {}", lineno + 1));
                }

                let name = parts
                    .get(1)
                    .ok_or_else(|| format!("Malformed DEF at line {}", lineno + 1))?
                    .to_string();

                current = Some(Symbol {
                    name,
                    strokes: Vec::new(),
                    left: 0,
                    right: 0,
                    anchors: HashMap::new(),
                });
            }

            "P" => {
                let glyph = current
                    .as_mut()
                    .ok_or_else(|| format!("P outside DEF at line {}", lineno + 1))?;

                let n: usize = parts
                    .get(1)
                    .ok_or_else(|| format!("Malformed P at line {}", lineno + 1))?
                    .parse()
                    .map_err(|_| format!("Invalid P count at line {}", lineno + 1))?;

                let mut stroke = Vec::with_capacity(n);

                for i in 0..n {
                    let xi = 5 + i * 2;
                    let yi = 6 + i * 2;

                    let x: i32 = parts
                        .get(xi)
                        .ok_or_else(|| format!("Missing X coord at line {}", lineno + 1))?
                        .parse()
                        .map_err(|_| format!("Invalid X coord at line {}", lineno + 1))?;

                    let y: i32 = parts
                        .get(yi)
                        .ok_or_else(|| format!("Missing Y coord at line {}", lineno + 1))?
                        .parse()
                        .map_err(|_| format!("Invalid Y coord at line {}", lineno + 1))?;

                    stroke.push((conv_x(x), conv_y(y)));
                }

                glyph.strokes.push(stroke);
            }

            "X" => {
                let glyph = current
                    .as_mut()
                    .ok_or_else(|| format!("X outside DEF at line {}", lineno + 1))?;

                let pin = parts
                    .get(1)
                    .ok_or_else(|| format!("Malformed X at line {}", lineno + 1))?
                    .to_string();

                let x: i32 = parts
                    .get(3)
                    .ok_or_else(|| format!("Missing X in X line {}", lineno + 1))?
                    .parse()
                    .map_err(|_| format!("Invalid X coord at line {}", lineno + 1))?;

                let y: i32 = parts
                    .get(4)
                    .ok_or_else(|| format!("Missing Y in X line {}", lineno + 1))?
                    .parse()
                    .map_err(|_| format!("Invalid Y coord at line {}", lineno + 1))?;

                let ax = conv_x(x);
                let ay = conv_y(y);

                glyph.anchors.insert(pin.clone(), (ax, ay));

                // detect if this pin represents the left or right boundary
                match pin.as_str() {
                    "~" => {
                        // ~ (unnamed) indicates either left or right based on sign
                        if ax > 0 {
                            glyph.right = ax;
                        } else {
                            glyph.left = ax;
                        }
                    }
                    "P" => {
                        // P always means left
                        glyph.left = ax;
                    }
                    "S" => {
                        // S always means right
                        glyph.right = ax;
                    }
                    _ => {}
                }
            }

            "ENDDEF" => {
                let glyph = current
                    .take()
                    .ok_or_else(|| format!("ENDDEF without DEF at line {}", lineno + 1))?;

                glyphs.insert(glyph.name.clone(), glyph);
            }

            _ => {}
        }
    }

    if current.is_some() {
        return Err("Unterminated DEF block".into());
    }

    Ok(glyphs)
}

/// A transformation that may be applied to a symbol when being composed into a glyph.
struct Transform {
    /// X-direction scale
    scale_x: i8,
    /// Y-direction scale
    scale_y: i8,
    /// Y-direction offset
    offset_y: i8,
}

const BASE: i8 = 9;
const CAP_HEIGHT: i8 = -21;
const X_HEIGHT: i8 = -14;
const SYM_HEIGHT: i8 = -16;
const SUP_OFFSET: i8 = -13;
const SUB_OFFSET: i8 = 6;

/// Split the transform prefix from a symbol name.
/// For instance, "!PARENTHESIS" becomes "!" and "PARENTHESIS".
fn split_transform(name: &str) -> (Transform, &str) {
    let first = name.chars().next();

    if let Some(c) = first {
        let transform = match c {
            '!' => Some(Transform {
                scale_x: -1,
                scale_y: 1,
                offset_y: 0,
            }),
            '-' => Some(Transform {
                scale_x: 1,
                scale_y: -1,
                offset_y: X_HEIGHT,
            }),
            '=' => Some(Transform {
                scale_x: 1,
                scale_y: -1,
                offset_y: CAP_HEIGHT,
            }),
            '~' => Some(Transform {
                scale_x: 1,
                scale_y: -1,
                offset_y: SYM_HEIGHT,
            }),
            '+' => Some(Transform {
                scale_x: -1,
                scale_y: -1,
                offset_y: X_HEIGHT,
            }),
            '%' => Some(Transform {
                scale_x: -1,
                scale_y: -1,
                offset_y: CAP_HEIGHT,
            }),
            '*' => Some(Transform {
                scale_x: -1,
                scale_y: -1,
                offset_y: SYM_HEIGHT,
            }),
            '^' => Some(Transform {
                scale_x: 1,
                scale_y: 1,
                offset_y: SUP_OFFSET,
            }),
            '`' => Some(Transform {
                scale_x: -1,
                scale_y: 1,
                offset_y: SUP_OFFSET,
            }),
            '.' => Some(Transform {
                scale_x: 1,
                scale_y: 1,
                offset_y: SUB_OFFSET,
            }),
            ',' => Some(Transform {
                scale_x: -1,
                scale_y: 1,
                offset_y: SUB_OFFSET,
            }),
            _ => None,
        };

        if let Some(t) = transform {
            // first char is a transform symbol → strip it
            return (t, &name[c.len_utf8()..]);
        }
    }

    // first char is not a transform → return identity transform, full string
    (
        Transform {
            scale_x: 1,
            scale_y: 1,
            offset_y: 0,
        },
        name,
    )
}

/// Render the given symbol, with provided transform and offset applied
fn render_glyph(raw: &Symbol, tr: &Transform, offset_x: i8, offset_y: i8) -> Vec<PackedPoint> {
    let mut out = Vec::new();

    for stroke in &raw.strokes {
        let mut first_point = true;

        for &(x, y) in stroke {
            let px = x * tr.scale_x + offset_x;
            let py = y * tr.scale_y + offset_y + BASE;

            out.push(PackedPoint {
                x: px,
                y: py,
                pen: !first_point,
            });

            first_point = false;
        }
    }

    out
}

/// Transform the left and right metrics of the given symbol.
fn transform_metrics(raw: &Symbol, tr: &Transform) -> (i8, i8) {
    let (l, r) = (raw.left, raw.right);

    if tr.scale_x >= 0 { (l, r) } else { (-r, -l) }
}

/// Build a glyph from a single symbol name.
fn build_single(raw: &HashMap<String, Symbol>, name: &str) -> Option<Glyph> {
    let (tr, base_name) = split_transform(name);
    if let Some(base) = &raw.get(base_name) {
        let strokes = render_glyph(base, &tr, 0, 0);
        let (left, right) = transform_metrics(base, &tr);

        Some(Glyph {
            left,
            right,
            strokes,
        })
    } else {
        eprintln!("Failed to find glyph for name: {}", base_name);
        None
    }
}

/// Compute the offset used when placing an accent character on a base character.
fn anchor_offset(
    base: &Symbol,
    accent: &Symbol,
    anchor: Option<&str>,
    base_tr: &Transform,
    accent_tr: &Transform,
) -> (i8, i8) {
    // No anchor → zero offset
    let anchor = match anchor {
        Some(a) => a,
        None => return (0, 0),
    };

    // Split at '=' if present: left = base key, right = accent key (modifier)
    let mut parts = anchor.splitn(2, '=');
    let base_key = parts.next().unwrap(); // e.g., "ABOVE"
    let accent_key = parts.next().unwrap_or(base_key); // e.g., "X" or fallback to base_key

    // Lookup coordinates in glyph anchors
    let (bx, by) = base.anchors.get(base_key).copied().unwrap();
    let (ax, ay) = accent.anchors.get(accent_key).copied().unwrap_or((0, 0));

    // Apply transforms
    let ox = bx * base_tr.scale_x - ax * accent_tr.scale_x;
    let oy = by * base_tr.scale_y + base_tr.offset_y - ay * accent_tr.scale_y - accent_tr.offset_y;

    (ox, oy)
}

/// Create a glyph by composing two characters.
fn compose_two(raw: &HashMap<String, Symbol>, a: &str, b: &str) -> Option<Glyph> {
    let (ta, a_name) = split_transform(a);

    // Split B into accent and optional anchor
    let mut b_parts = b.splitn(2, ' ');
    let acc_glyph_name = b_parts.next().unwrap();
    let anchor = b_parts.next(); // e.g. Some("ABOVE=X")

    let (tb, b_name) = split_transform(acc_glyph_name);

    let base = raw.get(a_name).expect(&format!(
        "Failed to find glyph for A name: {} (combining with B name: {})",
        a_name, b_name
    ));

    let acc = raw.get(b_name).expect(&format!(
        "Failed to find glyph for B name: {} (combining with A name: {})",
        b_name, a_name
    ));

    // Pass anchor to anchor_offset
    let (ox, oy) = anchor_offset(base, acc, anchor, &ta, &tb);

    let mut strokes = render_glyph(base, &ta, 0, 0);
    strokes.extend(render_glyph(acc, &tb, ox, oy));

    let (l1, r1) = transform_metrics(base, &ta);
    let (l2, r2) = transform_metrics(acc, &tb);

    Some(Glyph {
        left: l1.min(l2 + ox),
        right: r1.max(r2 + ox),
        strokes,
    })
}

/// Parse the "charlist.txt" file, containing the mapping from Unicode codepoint to KiCAD symbol.
fn parse_charlist(input: &str, font: &HashMap<String, Symbol>) -> FontFile {
    let mut out: FontFile = std::array::from_fn(|_| None);

    let mut codepoint: usize = 0;

    for (lineno, line) in input.lines().enumerate() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        // remove trailing comment if present
        let line = line.splitn(2, '#').next().unwrap().trim();

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "startchar" => {
                codepoint = parts
                    .get(1)
                    .expect("missing startchar value")
                    .parse::<usize>()
                    .expect("invalid startchar value");

                if codepoint >= NUM_GLYPHS {
                    panic!("startchar {} out of range", codepoint);
                }
            }

            "font" => {
                // ignore, only one font output
            }

            "+" => {
                if codepoint >= NUM_GLYPHS {
                    continue;
                }

                let rest = line[1..].trim_start();

                // Split on any amount of whitespace
                let parts: Vec<&str> = rest.split_whitespace().collect();

                let first_glyph = parts.get(0).expect("missing base glyph");
                let second_glyph = if parts.len() > 1 {
                    Some(parts[1..].join(" ")) // join everything after the first
                } else {
                    None
                };

                let glyph = match second_glyph {
                    Some(second) => compose_two(font, first_glyph, &second),
                    None => build_single(font, first_glyph),
                };

                out[codepoint] = Some(glyph.expect("failed to create glyph"));
                codepoint += 1;
            }

            "+w" | "+p" => {
                eprintln!("unsupported + form at line {}: {}", lineno + 1, line);
                codepoint += 1;
            }

            "+(" => {
                // opening group (?)
                eprintln!("unsupported + form at line {}: {}", lineno + 1, line);
                codepoint += 1;
            }

            "+|" | "+)" => {
                // continuing/closing group (?)
                eprintln!("unsupported + form at line {}: {}", lineno + 1, line);
            }

            "//" => {
                // ignore, this is a comment
            }

            "skipcodes" => {
                codepoint += parts
                    .get(1)
                    .expect("missing skipcodes value")
                    .parse::<usize>()
                    .expect("invalid skipcodes value");
            }

            _ => {
                panic!("unsupported command at line {}: {}", lineno + 1, line);
            }
        }
    }

    out
}

fn main() {
    let mut symbols = parse_lib_file(&fs::read_to_string("data/font.lib").unwrap()).unwrap();
    symbols.extend(parse_lib_file(&fs::read_to_string("data/symbol.lib").unwrap()).unwrap());

    let glyphs = parse_charlist(&fs::read_to_string("data/charlist.txt").unwrap(), &symbols);

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let out_file = out_dir.join("newstroke_font.rs");

    fs::write(out_file, generate_rust(&glyphs)).unwrap();

    println!("cargo:rerun-if-changed=data/charlist.txt");
    println!("cargo:rerun-if-changed=data/CJK.lib");
    println!("cargo:rerun-if-changed=data/font.lib");
    println!("cargo:rerun-if-changed=data/symbol.lib");
}
