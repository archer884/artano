# Artano

A small Rust library for adding styled text to images — specifically, the
"Impact" font top/bottom caption meme style. Named after Sauron (Quenya
for "High Smith"), per the author's sense of humor.

## At a glance

- **Crate:** `artano` v0.3.14, edition 2024, dual-licensed MIT/Apache-2.0
- **Purpose:** Render an image with caption text overlaid in the classic
  white-on-black-outline meme style.
- **Backbone:** `image` 0.25, `imageproc` 0.27, `rusttype` 0.9, `font-kit` 0.14
- **Test surface:** Tiny — one `load_font("Impact")` smoke test, one
  `split_text` test, no integration tests, no fixtures.
- **Dev tools:** `examples/benchmark.rs` (read/annotate/render/total
  timings across image sizes and annotation counts) and
  `examples/diff.rs` (pixel-wise diff between two render methods with
  amplified visualizations in `target/diff/`).

## Public API surface (`src/lib.rs`)

Re-exports everything from the submodules plus `rusttype::Font`:

- `Canvas` (`src/canvas.rs`) — the main entry point. Holds a base image
  and a `Vec<RenderedLayer>` of small text-bbox buffers; `render()`
  resizes each layer individually rather than the whole image.
- `Annotation`, `Position::{Top,Middle,Bottom}`, `PlacedLine`
  (`src/annotation.rs`) — text + vertical placement, plus a pure
  `Annotation::layout()` that returns `Vec<PlacedLine>` (no rendering).
- `Error` (`src/error.rs`) — wraps `io::Error`, `image::ImageError`,
  `font_kit::SelectionError`, and a stringly-typed font-not-found variant.
- `outline` (`src/outline.rs`, `pub`) — the current outline renderer
  (text + black halo via a single morphology dilate).
- `legacy` (`src/legacy.rs`, `pub`) — the *original* (pre-refactor)
  pipeline preserved verbatim: one full-image 3× overlay, Canny edge
  detection + per-pixel hollow circles for the outline, one big
  Lanczos3 resize. `legacy::render(bytes, &[Annotation], &Font) -> Result<DynamicImage>`
  reproduces the pre-refactor behavior end-to-end. The diff binary
  uses it as the reference for visual comparison.
- `load_font(name: &str) -> Result<Font>` — loads a system font by
  PostScript name via `font-kit`. Expects Impact to be installed (the
  only test asserts this on the host).
- `Result<T>` = `std::result::Result<T, Error>`.
- Constants `AA_FACTOR: u32 = 3` and `AA_FACTOR_FLOAT: f32 = 3.0` (the
  supersampling factor used for antialiasing — referenced in multiple
  places, so don't rename without grepping).

## How rendering works (the mental model)

1. `Canvas::read_from_buffer(&[u8])` decodes an image (any format `image`
   supports) into `base`, with `layers: Vec<RenderedLayer>` empty.
2. `Canvas::add_annotation(&annotation, &font, scale_multiplier)` lays
   out the text at `height/10 * scale_multiplier` glyph height. The
   layout (splitting, line offsets per `Position` variant) lives in
   `Annotation::layout` and returns `Vec<PlacedLine>` — pure, no drawing.
3. For each `PlacedLine`, `render_line_to_small_buffer` (in
   `src/canvas.rs`) allocates a small RGBA buffer sized to
   `(text_width * AA_FACTOR + 2*pad, text_height * AA_FACTOR + 2*pad)`
   where `pad = (outline_radius + lanczos_kernel_radius).div_ceil(AA_FACTOR) * AA_FACTOR`
   covers both the outline radius and the Lanczos3 kernel's 3-pixel
   output radius (9 input pixels at 3×). `outline::render_line` then
   produces text + outline into that buffer; the layer is pushed to
   `self.layers`.
4. `outline::render_line` (the current path):
   1. Draw the text mask (white text on black) at 3× into a `GrayImage`.
   2. Build a 1-pixel-thick **ring** mask at the outline radius
      `(0.09 * scale_factor) as i32`.
   3. `imageproc::morphology::grayscale_dilate(text_mask, ring_mask)` →
      a 1-pixel ring at that radius. Same math as the old
      `draw_hollow_circle_mut`-per-edge-pixel approach, but one dilate
      pass over the whole buffer.
   4. Stamp the ring as opaque black into the small buffer.
   5. Draw the white text on top via `draw::text` (weighted blend, so
      the AA edges transition smoothly out of the black outline).
5. `Canvas::render()` pops each layer, `imageops::resize`s it from
   `AA_FACTOR`× down via Lanczos3, and `imageops::overlay`s it onto
   `base` at `(line.x - pad/AA_FACTOR, line.y - pad/AA_FACTOR)` so the
   text lands at the original `(x, y)` and the outline extends ~3 px
   outside the text bbox in each direction.
6. `Canvas::save_jpg` / `save_png` write the result.

Note `Canvas` is mutated through `&mut self` for both `add_annotation`
and `render`.

## The legacy pipeline (kept for diffing)

`legacy::render` runs the *original* path end-to-end:
- One `RgbaImage` sized to `(width * AA_FACTOR, height * AA_FACTOR)`.
- Each annotation's text + outline is drawn into that overlay via
  `legacy::render_line` — Canny edge detection on a 3× Luma buffer,
  then `draw_hollow_circle_mut` at every edge pixel with radius
  `(0.09 * scale_factor)`, then white text on top.
- One `imageops::resize` of the whole 3× overlay with Lanczos3, then
  `imageops::overlay` onto the base.

The `examples/diff.rs` binary calls `legacy::render` for `render_b` and
the current `Canvas` API for `render_a`; on the same input the diff
should be effectively zero (sub-pixel rounding from the dilate
discretization). See "Common tasks" below for the exact command.

## Common tasks

- **Add a new text position** (e.g. `TopLeft`): extend `Position` in
  `src/annotation.rs`, add a constructor and a `position::*` helper,
  then add a match arm in `Annotation::layout` (the three existing
  arms already call `PlacedLine { ... y: pos.1 + line_offset, ... }`
  with different `line_offset`s when text is split). `Position::position()`
  is now `pub` so it can be reused outside this module.
- **Tweak the outline**: `outline::render_line` in `src/outline.rs` is
  the only place. The ring-kernel radius formula is
  `(0.09 * scale_factor) as i32`. The padding in
  `Canvas::render_line_to_small_buffer` adds a `9 px` (Lanczos3
  output radius at 3×) margin on top of that. If you change the
  outline radius, also revisit the pad.
- **Add a new output format**: `Canvas::save_*` are one-liners that
  delegate to `image::DynamicImage::write_to`. Add another method
  following the same pattern.
- **Use a font not on the system**: `load_font` won't work. The
  `rusttype::Font::try_from_vec_and_index` path it uses internally is
  what you want — bypass `load_font` and feed bytes directly.

## Build / verify

```sh
cargo build
cargo test                       # Impact smoke test + split_text
cargo clippy                     # should be warning-free

cargo run --release --example benchmark
                                 # per-case read/annotate/render/total timings
cargo run --release --example diff
                                 # renders each case via both methods and reports
                                 # max/mean/pixel-count diffs; writes amplified
                                 # |a-b|*16 visualizations to target/diff/<case>.png
```

The Impact font test will fail on systems without Impact installed
(most Linux). That's expected — it's a smoke test, not a CI gate
worth chasing. On macOS it's preinstalled.

## Repo layout

```
Cargo.toml              package + deps
src/lib.rs              module wiring, load_font, Result alias, AA constants
src/canvas.rs           Canvas: base + Vec<RenderedLayer>, per-layer render
src/annotation.rs       Annotation, Position, pure layout() -> Vec<PlacedLine>
src/outline.rs          current outline: text mask + 1-px ring dilate
src/legacy.rs           original pipeline (one 3× overlay + Canny + circles)
src/draw.rs             thin wrapper over rusttype's glyph draw loop with weighted blending
src/error.rs            Error enum + From impls
examples/benchmark.rs   read/annotate/render/total timings
examples/diff.rs        method_a vs method_b pixel diff + amplified visualization
AGENTS.md               this file
```

## Gotchas / non-obvious things

- **`AA_FACTOR` lives in two forms** (`u32` and `f32`) because it's
  used in both integer pixel math and float `Scale` math. If you
  change one, change both.
- **`legacy::split_text` panics** with `"Wtf, bro?..."` if the input has
  no space. The caller in `Annotation::layout` already guards on
  `self.text.contains(' ')`, so the panic is unreachable in practice —
  but if you refactor, preserve that invariant or replace the panic
  with a real error.
- **`outline::render_line`'s `root_position` is in *base* coordinates**,
  not buffer coordinates. Internally it multiplies by `AA_FACTOR` to
  get the buffer position. The call site in
  `Canvas::render_line_to_small_buffer` passes
  `(pad / AA_FACTOR, pad / AA_FACTOR)` so the text lands at
  `(pad, pad)` in the small buffer. Passing `(pad, pad)` directly
  draws at `(pad * AA_FACTOR, pad * AA_FACTOR)` and silently misaligns
  the whole composition — bug I hit during the refactor, easy to
  re-hit.
- **`render_line_to_small_buffer`'s pad** = `outline_radius + 9` px (the
  Lanczos3 kernel's 3-px output radius × 3 = 9 input px), rounded up
  to a multiple of `AA_FACTOR` so the 3× resize and the composite
  offset come out exact. Both the outline and the resize kernel need
  to fit inside this pad or you get clipping at the AA edges.
- **`Cargo.lock` is gitignored** (unusual for a binary-adjacent lib,
  but the author prefers it that way for libraries).
- **Edition is 2024** — requires a recent rustc (1.85+). The
  toolchain on this machine (1.97) is fine.
- **`conv = "0.3.3"` is a declared dependency but is not used in
  `src/`** (no `use conv;` anywhere). Looks vestigial; safe to drop
  if you ever tidy deps.
- **No `#[non_exhaustive]` on `Error` or `Position`** — adding
  variants is a breaking change for downstream users.
- **The `imageproc::morphology::Mask` constructor is `from_image`** (with
  private `new`); the max image dimension is 511 pixels. That's fine
  for outline radii up to ~250 (i.e. images up to ~25000 px tall at
  the default `height/10` scale), but worth knowing if you ever
  crank `scale_multiplier`.
