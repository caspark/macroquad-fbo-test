# macroquad-fbo-test

A benchmark for comparing rendering performance in [macroquad](https://github.com/not-fl3/macroquad) when drawing sprites directly to the default framebuffer vs. rendering through intermediate FBOs (framebuffer objects / render targets) with optional compositing.

This was created to investigate an FBO-related performance regression where rendering through render targets was significantly slower than drawing directly to the screen.

## What it does

Draws a configurable number of animated sprites in a spiral pattern using procedurally generated textures, then reports frame time statistics (avg FPS, P50/P95/P99 frame times).

Three rendering modes can be compared:

1. **Default framebuffer** — sprites drawn directly to the screen
2. **FBO** (`--fbo`) — sprites drawn to a scene render target, background drawn to a separate render target, both blitted to screen (mimics a typical game rendering pipeline)
3. **FBO + composite shader** (`--fbo --composite`) — same as above but uses a multi-texture compositing shader to combine the render targets, similar to the pipeline used in [Signs of Danger](https://store.steampowered.com/app/3816900/Signs_of_Danger/)

## Usage

```
cargo run --release -- [OPTIONS]
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--fbo` | Render through FBOs instead of the default framebuffer | off |
| `--composite` | Use a compositing shader (implies FBO pipeline) | off |
| `--sprites <N>` | Number of sprites to draw per frame | 500 |
| `--textures <N>` | Number of distinct textures to cycle through (more = more batch breaks) | 16 |
| `--scale <F>` | Camera zoom scale factor | 3.0 |
| `--duration <F>` | How long to run the benchmark in seconds | 4 |

### Example

```sh
# Fast (default framebuffer) — should be ~60 FPS regardless of patch
cargo run --release -- --sprites 2000 --textures 16

# Slow with upstream macroquad (FBO render target) — ~3 FPS with 2000 sprites/16 textures
# Fast (~60 FPS) after uncommenting the patch in Cargo.toml
cargo run --release -- --sprites 2000 --textures 16 --fbo

# Same as above but with compositing shader
cargo run --release -- --sprites 2000 --textures 16 --fbo --composite
```

## The bug

Upstream macroquad 0.4.14's `render_target_ex()` creates unnecessary MSAA resolve textures even for non-multisampled render targets (sample_count=1). This causes an MSAA blit on every `end_render_pass()`, resulting in O(n) GPU pipeline flushes per frame where n = number of draw call batches.

With 1 texture everything batches into 1 draw call and FBO matches default FB performance. With multiple textures, each texture switch creates a new draw call batch, and the unnecessary MSAA resolve destroys performance.

## The fix

Uncomment the `[patch.crates-io]` section in `Cargo.toml` to use the fixed [macroquad](https://github.com/caspark/macroquad/tree/master-caspark-2026-02-19) and [miniquad](https://github.com/caspark/miniquad/tree/master-caspark-2026-02-19) forks.

macroquad fixes:

1. [`975aa6a`](https://github.com/caspark/macroquad/commit/975aa6a) — Skip MSAA resolve for non-multisampled render targets. `render_target_ex()` checked `sample_count != 0` but the default is 1 (not 0), so resolve targets were always created, causing N full-framebuffer `glBlitFramebuffer` calls per frame. Fix: change `!= 0` to `> 1`.
2. [`7c0e871`](https://github.com/caspark/macroquad/commit/7c0e871) — Prevent double-delete of non-MSAA render target textures. The GL texture ID was owned by both the render pass and `Texture2D`, causing double-free when the ID got reused. Fix: use `Texture2D::unmanaged()` for non-MSAA render targets.
3. [`7473683`](https://github.com/caspark/macroquad/commit/7473683) — Proper texture ownership for render targets, replacing the unmanaged workaround with `Arc` ref-counting for all user-facing `Texture2D` values. Also fixes resolve FBO leak on MSAA render target drop.

miniquad fix:

1. [`d93760c`](https://github.com/caspark/miniquad/commit/d93760c) — `delete_render_pass` no longer deletes color/depth textures (texture lifetime is now caller's responsibility, matching Metal backend). Also fixes resolve FBO leak.

## Benchmark results

On AMD integrated GPU (1920x1080, scale 3.0):

| Sprites | Textures | Default FB | FBO (upstream) | FBO (patched) | Speedup |
|---------|----------|------------|----------------|---------------|---------|
| 2000    | 1        | 60 FPS     | 60 FPS         | 60 FPS        | 1x      |
| 2000    | 16       | 60 FPS     | 3 FPS          | 60 FPS        | 20x     |
| 25000   | 1        | 60 FPS     | 60 FPS         | 60 FPS        | 1x      |
| 25000   | 16       | 38 FPS     | 0.2 FPS        | 29 FPS        | ~145x   |

## Dependencies

- [macroquad](https://github.com/not-fl3/macroquad) — graphics/windowing
- [clap](https://github.com/clap-rs/clap) — CLI argument parsing
