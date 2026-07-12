# Artano

A small Rust library for adding styled text to images — specifically, the
"Impact" font top/bottom caption meme style. Named after Sauron (Quenya
for "High Smith"), per the author's sense of humor.

## At a glance

- **Crate:** `artano` v0.3.13, edition 2024, dual-licensed MIT/Apache-2.0
- **Purpose:** Render an image with caption text overlaid in the classic
  white-on-black-outline meme style.
- **Backbone:** `image` 0.25, `imageproc` 0.26, `rusttype` 0.9, `font-kit` 0.14
- **Test surface:** Tiny — one `load_font("Impact")` smoke test and one
  `split_text` test. No integration tests, no fixtures, no example binary.

## Public API surface (`src/lib.rs`)

Re-exports everything from the four submodules plus `rusttype::Font`:

- `Canvas` (`src/canvas.rs`) — the main entry point. Holds a base image
  and a 3x-upscaled overlay image used to render text at high resolution
  for antialiasing before downsampling.
- `Annotation`, `Position::{Top,Middle,Bottom}` (`src/annotation.rs`) —
  the text + vertical placement to draw.
- `Error` (`src/error.rs`) — wraps `io::Error`, `image::ImageError`,
  `font_kit::SelectionError`, and a stringly-typed font-not-found variant.
- `load_font(name: &str) -> Result<Font>` — loads a system font by
  PostScript name via `font-kit`. Expects Impact to be installed (the
  only test asserts this on the host).
- `Result<T>` = `std::result::Result<T, Error>`.
- Constants `AA_FACTOR: u32 = 3` and `AA_FACTOR_FLOAT: f32 = 3.0` (the
  supersampling factor used for antialiasing — referenced in multiple
  places, so don't rename without grepping).

## How rendering works (the mental model)

1. `Canvas::read_from_buffer(&[u8])` decodes an image (any format `image`
   supports) and creates a 3x overlay `RgbaImage` of the same scene.
2. `Canvas::add_annotation(&annotation, &font, scale_multiplier)` lays
   out the text at `height/10 * scale_multiplier` glyph height and
   renders it onto the overlay.
3. `Annotation::render_text` handles the placement logic. If the text
   would exceed image width (`> 1.2 * c_width`) and contains a space,
   it splits at the space nearest the midpoint and renders two lines
   (the split offsets differ per `Position` variant). Otherwise a
   single line is rendered.
4. `render_line` (in `annotation.rs`) is where the meme look comes
   from: it draws white text onto a temporary edge-detection buffer,
   runs `imageproc::edges::canny`, then paints a black hollow circle
   at every detected edge pixel before drawing the white text on top.
   The radius of that outline is `(0.09 * scale_factor)`.
5. `Canvas::render()` Lanczos3-downsamples the overlay onto the base.
6. `Canvas::save_jpg` / `save_png` write the result.

Note `Canvas` is mutated through `&mut self` for both `add_annotation`
and `render` — methods aren't `&mut self` compatible with shared refs
in awkward ways, so be careful with borrowing.

## Common tasks

- **Add a new text position** (e.g. `TopLeft`): extend `Position` in
  `src/annotation.rs`, add a constructor and a `position::*` helper,
  then update the match in `Annotation::render_text` (all three arms
  currently handle `Top`/`Middle`/`Bottom` and call `render_line`
  twice with different `y_offset`s when text is split).
- **Tweak the outline**: `render_line` in `src/annotation.rs` is the
  only place. The Canny thresholds are hardcoded to `255.0, 255.0`
  (essentially "any non-zero gradient"). The radius formula is
  `(0.09 * scale_factor) as i32`.
- **Add a new output format**: `Canvas::save_*` are one-liners that
  delegate to `image::DynamicImage::write_to`. Add another method
  following the same pattern.
- **Use a font not on the system**: `load_font` won't work. The
  `rusttype::Font::try_from_vec_and_index` path it uses internally is
  what you want — bypass `load_font` and feed bytes directly.

## Build / verify

```sh
cargo build
cargo test     # runs the Impact smoke test and split_text test
cargo clippy   # if you want the lints
```

The Impact font test will fail on systems without Impact installed
(most Linux). That's expected — it's a smoke test, not a CI gate
worth chasing. On macOS it's preinstalled.

## Repo layout

```
Cargo.toml          package + deps
src/lib.rs          module wiring, load_font, Result alias, AA constants
src/canvas.rs       Canvas: base + overlay, render, save_jpg/png
src/annotation.rs   Annotation, Position, text layout/split, outline render
src/draw.rs         thin wrapper over rusttype's glyph draw loop with weighted blending
src/error.rs        Error enum + From impls
```

## Gotchas / non-obvious things

- **`AA_FACTOR` lives in two forms** (`u32` and `f32`) because it's
  used in both integer pixel math and float `Scale` math. If you
  change one, change both.
- **`split_text` panics** with `"Wtf, bro?..."` if the input has no
  space. The caller in `render_text` already guards on
  `self.text.contains(' ')`, so the panic is unreachable in practice —
  but if you refactor, preserve that invariant or replace the panic
  with a real error.
- **The README's license links are swapped**: `[LICENSE-APACHE]` points
  at `LICENSE-MIT` and vice versa. Real files are fine; only the
  markdown links in `README.md` are wrong. Not worth fixing unless
  you're already in the file.
- **`Cargo.lock` is gitignored** (unusual for a binary-adjacent lib,
  but the author prefers it that way for libraries).
- **Edition is 2024** — requires a recent rustc (1.85+). The
  toolchain on this machine (1.97) is fine.
- **`conv = "0.3.3"` is a declared dependency but is not used in
  `src/`** (no `use conv;` anywhere). Looks vestigial; safe to drop
  if you ever tidy deps.
- **No `#[non_exhaustive]` on `Error` or `Position`** — adding
  variants is a breaking change for downstream users.
