# The `vector-text` crate

`vector-text` is a library for drawing text to a vector output
using various vector-based fonts.

This can be used for drawing text to plotters, with laser displays, on
XY oscilloscopes, or for other purposes!

The library supports `no_std` environments but requires an allocator.

Supported fonts include:

- [BGI (Borland)](https://moddingwiki.shikadi.net/wiki/BGI_Stroked_Font) fonts including `LITT.CHR`, via [vector_text_borland]
- [Hershey](https://paulbourke.net/dataformats/hershey/) fonts, via [vector_text_hershey]
- The [NewStroke](https://vovanium.ru/sledy/newstroke/en) font, via [vector_text_newstroke]

This library provides the render_text function which you can use to render text, e.g.:

```rust
use vector_text::{render_text, VectorFont, HersheyFont};

let result = render_text("Hello World!", VectorFont::HersheyFont(HersheyFont::Romans));
```
