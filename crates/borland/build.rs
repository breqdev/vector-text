use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

#[derive(Debug, Copy, Clone)]
struct PackedPoint {
    pub x: i8,
    pub y: i8,
    pub pen: bool,
}

const NUM_GLYPHS: usize = 256; // ASCII only, sorry
type FontFile = [Option<Glyph>; NUM_GLYPHS];

/// Generate the output Rust definitions for the font data.
fn generate_rust(font: &[Option<Glyph>], name: &str) -> String {
    let mut out = String::new();

    // Write the symbol table
    out.push_str(&format!(
        "static {}_FONT: [Option<Glyph>; {}] = [\n",
        name,
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

/// Represents a position that may be advanced within a buffer.
struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

struct PackedCoord {
    opcode: u8,
    x: i8,
    y: i8,
}

/// Parse the "7-bit signed integer" format used for X and Y coordinates.
fn parse_7bit_signed(input: u8) -> i8 {
    let input = input & 0x7F;

    if input & 0x40 != 0 {
        // Sign-extend the 7th bit into the 8th bit
        (input | 0x80) as i8
    } else {
        input as i8
    }
}

impl<'a> Cursor<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Read enough bytes to fill the provided buffer.
    fn read(&mut self, out: &mut [u8]) {
        let end = self.pos + out.len();
        if end > self.buf.len() {
            panic!();
        }
        out.copy_from_slice(&self.buf[self.pos..end]);
        self.pos = end;
    }

    /// Read a single byte from the input.
    fn read_u8(&mut self) -> u8 {
        let mut result = [0];
        self.read(&mut result);
        result[0]
    }

    /// Read a 16-bit little-endian integer ("word" in the format description).
    fn read_u16_le(&mut self) -> u16 {
        let mut result = [0, 0];
        self.read(&mut result);
        u16::from_le_bytes(result)
    }

    /// Skip past the following number of bytes.
    fn skip(&mut self, n: usize) {
        let end = self.pos + n;
        if end > self.buf.len() {
            panic!();
        }
        self.pos = end;
    }

    /// Skip to the provided location in the file.
    fn skip_to(&mut self, n: usize) {
        let end = n;
        if end > self.buf.len() {
            panic!("Cannot skip to {}, file has length {}", n, self.buf.len());
        }
        self.pos = end;
    }

    /// Read a packed coordinate (two-byte structure containing X, Y, and a 2-bit opcode).
    fn read_coord(&mut self) -> PackedCoord {
        let mut data = [0, 0];
        self.read(&mut data);

        let op1 = (data[0] >> 7) & 0b1;
        let op2 = (data[1] >> 7) & 0b1;

        let x_twos = data[0] & 0b01111111;
        let y_twos = data[1] & 0b01111111;

        let x = parse_7bit_signed(x_twos);
        let y = -parse_7bit_signed(y_twos);

        PackedCoord {
            opcode: op1 << 1 | op2,
            x,
            y,
        }
    }
}

/// Parse a .CHR format font file.
///
/// Based on this specification:
/// https://www.fileformat.info/format/borland-chr/corion.htm
fn parse_chrfile(input: &[u8]) -> FontFile {
    let mut cur = Cursor::new(input);

    // Read file magic
    let mut magic = [0; 8];
    cur.read(&mut magic);

    assert_eq!(magic, [b'P', b'K', 0x08, 0x08, b'B', b'G', b'I', b' ']);

    // Read font desc until chr 26
    let mut desc = Vec::new();

    loop {
        let chr = cur.read_u8();

        if chr == 26 {
            break;
        } else {
            desc.push(chr);
        }
    }

    let desc = String::from_utf8(desc).unwrap();
    eprintln!("Loaded font: {}", desc);

    // Header length
    let header_len = cur.read_u16_le();
    eprintln!("Header len: {}", header_len);

    // Short font name
    let mut name = [0; 4];
    cur.read(&mut name);
    let name = str::from_utf8(&name).unwrap();
    eprintln!("Short name: {}", name);

    // More info
    let _file_size = cur.read_u16_le();
    let _driver_major_version = cur.read_u8();
    let _driver_minor_version = cur.read_u8();

    let header_end = cur.read_u16_le();
    // docs list this as 0x0100 but i think that's an endianness oops
    assert_eq!(header_end, 0x0001);

    // Seek to end of header
    eprintln!("Skipping to {}", header_len);
    cur.skip_to(header_len as usize);

    // Parse font details
    let signature = cur.read_u8();
    assert_eq!(signature, b'+');

    let num_characters = cur.read_u16_le();
    eprintln!("{} characters in file", num_characters);

    cur.skip(1);
    let start_char = cur.read_u8();
    eprintln!("Starting at character {}", start_char);

    // TODO: what does this mean?
    let _stroke_offset = cur.read_u16_le();

    let _scan_flag = cur.read_u8(); // docs say "??" so idk what this is

    // Font metric time!
    // Distance from origin to top of capital letter
    let _origin_to_top = cur.read_u8();
    // Distance from origin to baseline
    let _origin_to_baseline = cur.read_u8();
    // Distance from origin to bottom of descender
    let _origin_to_descender = cur.read_u8();

    // Docs specify that this is the short font name, repeated
    // Nope -- null bytes! At least in my file
    cur.skip(4);

    // there is an extra byte here that they forgot about in the spec
    cur.skip(1);

    assert_eq!(cur.pos, 0x0090);

    // Offsets to stroke data for each character
    // TODO there is surely a faster way lol
    let mut chr_offsets = Vec::with_capacity(num_characters as usize);

    for _ in 0..num_characters {
        let offset = cur.read_u16_le();
        chr_offsets.push(offset);
    }

    // Width of each character
    let mut chr_widths = Vec::with_capacity(num_characters as usize);
    for _ in 0..num_characters {
        let width = cur.read_u8();
        chr_widths.push(width);
    }

    // The rest of the file is character definitions!

    let data_section_start = cur.pos;

    let mut file: FontFile = std::array::from_fn(|_| None);

    for i in 0..(num_characters as usize) {
        let ascii_value = i + start_char as usize;
        let offset = chr_offsets[i] as usize + data_section_start;
        let width = chr_widths[i];

        cur.skip_to(offset);

        let mut path = Vec::new();

        loop {
            let coord = cur.read_coord();

            match coord.opcode {
                0b00 => {
                    // End of character definition
                    break;
                }
                0b01 => {
                    // "Do scan"
                    panic!("Unknown scan command");
                }
                0b10 => {
                    // Move the pointer to X, Y
                    path.push(PackedPoint {
                        x: coord.x,
                        y: coord.y,
                        pen: false,
                    });
                }
                0b11 => {
                    // Draw from the current pointer to X, Y
                    path.push(PackedPoint {
                        x: coord.x,
                        y: coord.y,
                        pen: true,
                    });
                }
                _ => unreachable!(),
            }
        }

        let glyph = Glyph {
            left: 0,
            right: (width as i8),
            strokes: path,
        };

        file[ascii_value] = Some(glyph);
    }

    file
}

/// Generate an enum and implementation mapping font names to glyph tables.
fn generate_enum(variants: &[&str]) -> String {
    let mut out = String::new();

    // Generate the enum definition
    out.push_str("/// A specific Borland font instance (i.e., `.CHR` file).\n");
    out.push_str("pub enum BorlandFont {\n");

    for font in variants {
        let name: String = font
            .chars()
            .enumerate()
            .map(|(i, c)| match i {
                0 => c.to_ascii_uppercase(),
                _ => c.to_ascii_lowercase(),
            })
            .collect();

        out.push_str(&format!("    {},\n", name));
    }

    out.push_str("}\n");

    // Generate implementation mapping to values
    out.push_str("impl BorlandFont {\n");
    out.push_str(&format!(
        "    fn table(self) -> &'static [Option<Glyph>; {}] {{\n",
        NUM_GLYPHS
    ));
    out.push_str("        match self {\n");

    for font in variants {
        let name: String = font
            .chars()
            .enumerate()
            .map(|(i, c)| match i {
                0 => c.to_ascii_uppercase(),
                _ => c.to_ascii_lowercase(),
            })
            .collect();
        out.push_str(&format!("            Self::{} => &{}_FONT,\n", name, font));
    }

    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n");

    out
}

fn main() {
    // TODO: "BOLD.CHR" does not parse properly
    let fonts = [
        // "BOLD",
        "EURO", "GOTH", "LCOM", "LITT", "SANS", "SCRI", "SIMP", "TRIP", "TSCR",
    ];

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let out_path = out_dir.join("chr_font.rs");

    let mut output = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&out_path)
        .unwrap();

    output.write_all(generate_enum(&fonts).as_bytes()).unwrap();

    for font in fonts {
        let glyphs = parse_chrfile(&fs::read(format!("data/{}.CHR", font)).unwrap());
        output
            .write_all(generate_rust(&glyphs, font).as_bytes())
            .unwrap();
        println!("cargo:rerun-if-changed=data/{}.CHR", font);
    }
}
