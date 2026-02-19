+++
title = "miniquad end_render_pass causes catastrophic FBO performance due to framebuffer ping-pong"
type = "bug"
priority = "p1"
created_at = "2026-02-19T06:29:33Z"
resolution = "done: Fixed with two macroquad commits on master-caspark-2026-02-19 branch:"
+++

miniquad `end_render_pass()` in `src/graphics/gl.rs` unconditionally calls `glBindFramebuffer(GL_FRAMEBUFFER, self.default_framebuffer)` after every draw call, even when the next draw call will immediately rebind the same FBO. This causes the GPU to ping-pong between the FBO and default framebuffer on every single draw call when rendering to a render target.

## Impact

Rendering N sprites with M different textures (M>1) to a render target (FBO) is ~100x slower than rendering the same sprites to the default framebuffer. The cost scales linearly with draw call count.

Benchmarks on AMD integrated GPU (1920x1080, scale 3.0):

| Sprites | Textures | Default FB | FBO     |
|---------|----------|------------|---------|
| 2000    | 1        | 60 FPS     | 60 FPS  |
| 2000    | 16       | 60 FPS     | 3 FPS   |
| 500     | 16       | 60 FPS     | 13 FPS  |
| 1000    | 16       | 60 FPS     | 6 FPS   |

With 1 texture everything batches into 1 draw call and FBO matches default FB. With 16 textures, each sprite is a separate draw call, and the framebuffer ping-pong destroys performance.

## Root cause in miniquad

File: `src/graphics/gl.rs`, function `end_render_pass` (around line 1676 in miniquad v0.4.14):

```rust
fn end_render_pass(&mut self) {
    unsafe {
        if let Some(pass) = self.cache.cur_pass.take() {
            let pass = &self.passes[pass.0];
            if let Some(resolves) = &pass.resolves {
                // ... MSAA resolve logic ...
            }
        }
        // THIS LINE is the problem — always rebinds default FB
        glBindFramebuffer(GL_FRAMEBUFFER, self.default_framebuffer);
        self.cache.bind_buffer(GL_ARRAY_BUFFER, 0, None);
        self.cache.bind_buffer(GL_ELEMENT_ARRAY_BUFFER, 0, None);
    }
}
```

Per draw call flow when rendering to FBO:
1. `begin_pass(Some(fbo))` → `glBindFramebuffer(GL_FRAMEBUFFER, fbo)`
2. draw
3. `end_render_pass()` → `glBindFramebuffer(GL_FRAMEBUFFER, default_fb)` ← unnecessary!
4. Next draw: `begin_pass(Some(fbo))` → `glBindFramebuffer(GL_FRAMEBUFFER, fbo)` again

Each framebuffer switch forces a GPU pipeline flush.

## Fix

The `glBindFramebuffer` to default in `end_render_pass` should either:
- Be removed entirely (let `begin_pass` handle all binding)
- Or be conditional: only bind default FB if there is no subsequent pass targeting the same FBO

The buffer unbinding (`bind_buffer(..., 0, None)`) may also be unnecessary and should be reviewed.

## Repro

This repo (`~/src/fbo-test/`) is a minimal macroquad repro. Run:

```bash
# Fast (default framebuffer)
cargo run --release -- --sprites 2000 --textures 16

# Slow (FBO render target)
cargo run --release -- --sprites 2000 --textures 16 --fbo
```

CLI flags: `--sprites N`, `--textures N`, `--fbo`, `--duration SECS`, `--scale F`

## Upstream context

This was discovered investigating a 24→18 FPS regression in the Signs of Danger game (wizard-pixels repo, charlie worktree). Commits `ea247b7a` and `8b9729c3` moved entity rendering from the default framebuffer to an explicit scene render target (needed for WGPU migration). The game has ~8 material/texture switches during entity rendering, creating enough draw calls to trigger this issue.

The miniquad fork is at the path referenced in the wizard-pixels Cargo.toml/lock. The fix should be made there.

## Resolution notes

Fixed with two macroquad commits on master-caspark-2026-02-19 branch:

1. 975aa6a - fix: skip MSAA resolve for non-multisampled render targets
   Changed sample_count check from != 0 to > 1 in render_target_ex().
   This eliminates unnecessary MSAA resolve blits (3 FPS → 60 FPS).

2. 046e230 - fix: prevent double-delete of non-MSAA render target textures
   The first fix caused render target textures to read back as empty in
   wizard-pixels because the same GL texture was owned by both the render
   pass (deleted by delete_render_pass) and Texture2D (garbage collected).
   GL would reuse the freed texture ID for new textures, which would then
   get double-deleted. Fix: use Texture2D::unmanaged() for non-MSAA render
   targets so only delete_render_pass handles GL texture cleanup.

Root cause: render_target_ex() with sample_count=1 created unnecessary
resolve textures and did MSAA blit on every end_render_pass, causing
O(n) GPU pipeline flushes per frame where n = number of draw call batches.

No miniquad changes were needed. Both fbo-test and wizard-pixels verified working.
