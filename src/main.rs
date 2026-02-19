use clap::Parser;
use macroquad::prelude::*;

#[derive(Parser)]
struct Args {
    /// Use scene render target (FBO) instead of default framebuffer
    #[arg(long)]
    fbo: bool,

    /// Use a compositing shader (like wizard-pixels) instead of direct draw_texture_ex
    #[arg(long)]
    composite: bool,

    /// Duration to run in seconds
    #[arg(long, default_value = "4")]
    duration: f32,

    /// Number of sprites to draw per frame
    #[arg(long, default_value = "500")]
    sprites: usize,

    /// Camera scale factor (like view.scale in the game)
    #[arg(long, default_value = "3.0")]
    scale: f32,

    /// Number of different textures to cycle through (more = more batch breaks)
    #[arg(long, default_value = "16")]
    textures: usize,
}

fn window_conf() -> Conf {
    Conf {
        window_title: "FBO Test".to_string(),
        window_width: 1920,
        window_height: 1080,
        fullscreen: false,
        ..Default::default()
    }
}

// Composite shader matching wizard-pixels structure: GLSL 300 es → 330, same uniform order
const COMPOSITE_VERTEX: &str = r#"#version 330
precision highp float;

in vec3 position;
in vec2 texcoord;
in vec4 color0;

out highp vec2 uv;
out highp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
"#;

const COMPOSITE_FRAGMENT: &str = r#"#version 330
precision highp float;

in vec2 uv;
out vec4 fragColor;

// Same uniform order as wizard-pixels composite.frag
uniform sampler2D SceneTexture;
uniform sampler2D AtomTexture;
uniform sampler2D LightingTexture;
uniform sampler2D BackgroundTexture;
uniform sampler2D BloomTexture;
uniform sampler2D TerrainSdfTexture;
uniform sampler2D LevelRawAtomsTexture;
uniform sampler2D LevelRawRgbasTexture;

void main() {
    // the screen texture is flipped vertically (same as wizard-pixels)
    vec2 screenUV = uv;
    screenUV.y = 1.0 - screenUV.y;

    vec4 bg = texture(BackgroundTexture, screenUV);
    vec4 scene = texture(SceneTexture, screenUV);
    // Composite: background behind, scene on top
    fragColor = vec4(mix(bg.rgb, scene.rgb, scene.a), max(bg.a, scene.a));
}
"#;

#[macroquad::main(window_conf)]
async fn main() {
    let args = Args::parse();
    let screen_res = vec2(screen_width(), screen_height());

    eprintln!(
        "Mode: {}{}, sprites: {}, textures: {}, scale: {}, screen: {}x{}",
        if args.fbo { "FBO" } else { "DEFAULT_FB" },
        if args.composite { "+COMPOSITE" } else { "" },
        args.sprites,
        args.textures,
        args.scale,
        screen_res.x,
        screen_res.y
    );

    let mut textures = Vec::new();
    for i in 0..args.textures {
        let mut img = Image::gen_image_color(32, 32, Color::new(0.0, 0.0, 0.0, 0.0));
        for y in 0..32u32 {
            for x in 0..32u32 {
                let r = ((x + i as u32 * 7) % 32) as f32 / 31.0;
                let g = ((y + i as u32 * 13) % 32) as f32 / 31.0;
                let b = (i as f32) / (args.textures as f32);
                img.set_pixel(x, y, Color::new(r, g, b, 0.9));
            }
        }
        let tex = Texture2D::from_image(&img);
        tex.set_filter(FilterMode::Nearest);
        textures.push(tex);
    }

    // Two render targets, like wizard-pixels' background_render_target and scene_render_target
    let background_rt = render_target(screen_res.x as u32, screen_res.y as u32);
    background_rt.texture.set_filter(FilterMode::Nearest);
    let scene_rt = render_target(screen_res.x as u32, screen_res.y as u32);
    scene_rt.texture.set_filter(FilterMode::Nearest);

    // Create compositing material (like wizard-pixels)
    let composite_material = if args.composite {
        Some(
            load_material(
                ShaderSource::Glsl {
                    vertex: COMPOSITE_VERTEX,
                    fragment: COMPOSITE_FRAGMENT,
                },
                MaterialParams {
                    textures: vec![
                        "BackgroundTexture".to_string(),
                        "AtomTexture".to_string(),
                        "LightingTexture".to_string(),
                        "BloomTexture".to_string(),
                        "TerrainSdfTexture".to_string(),
                        "LevelRawAtomsTexture".to_string(),
                        "LevelRawRgbasTexture".to_string(),
                        "SceneTexture".to_string(),
                    ],
                    ..Default::default()
                },
            )
            .unwrap(),
        )
    } else {
        None
    };

    // Create dummy textures for unused slots
    let dummy_tex = Texture2D::from_image(&Image::gen_image_color(1, 1, BLACK));

    let start_time = get_time();
    let mut frame_count = 0u64;
    let mut frame_times: Vec<f64> = Vec::new();
    let mut last_log_time = start_time;

    loop {
        let now = get_time();
        let elapsed = now - start_time;

        if elapsed >= args.duration as f64 {
            break;
        }

        let dt = get_frame_time();
        frame_times.push(dt as f64);
        frame_count += 1;

        if now - last_log_time >= 0.5 {
            let recent: Vec<&f64> = frame_times.iter().rev().take(30).collect();
            let avg_dt: f64 = recent.iter().copied().sum::<f64>() / recent.len() as f64;
            let fps = 1.0 / avg_dt;
            eprintln!("[{:.1}s] FPS: {:.1} (avg frame time: {:.2}ms)", elapsed, fps, avg_dt * 1000.0);
            last_log_time = now;
        }

        let world_visible = screen_res / args.scale;

        let game_camera = Camera2D {
            target: vec2(0.0, 0.0),
            zoom: vec2(
                1.0 / screen_res.x * 2.0 * args.scale,
                1.0 / screen_res.y * 2.0 * args.scale,
            ),
            ..Default::default()
        };

        if args.fbo {
            clear_background(Color::new(0.0, 0.0, 0.0, 0.0));

            // === STEP 1: Draw background to background_rt ===
            // (like wizard-pixels draws background gradient to background_render_target)
            {
                push_camera_state();
                let bg_cam = Camera2D {
                    target: game_camera.target,
                    zoom: vec2(game_camera.zoom.x, -game_camera.zoom.y),
                    render_target: Some(background_rt.clone()),
                    ..Default::default()
                };
                set_camera(&bg_cam);
                clear_background(Color::new(0.0, 0.0, 0.0, 0.0));

                // Draw a gradient background
                let half_w = world_visible.x / 2.0;
                let half_h = world_visible.y / 2.0;
                for i in 0..20 {
                    let t = i as f32 / 19.0;
                    let y = -half_h + t * world_visible.y;
                    let color = Color::new(0.3 * (1.0 - t), 0.1, 0.3 * t + 0.2, 1.0);
                    draw_rectangle(-half_w, y, world_visible.x, world_visible.y / 20.0, color);
                }

                pop_camera_state();
            }

            // === STEP 2: Draw sprites to scene_rt ===
            // (like wizard-pixels draws entities to scene_render_target)
            {
                push_camera_state();
                let scene_cam = Camera2D {
                    target: game_camera.target,
                    zoom: vec2(game_camera.zoom.x, -game_camera.zoom.y),
                    render_target: Some(scene_rt.clone()),
                    ..Default::default()
                };
                set_camera(&scene_cam);
                clear_background(Color::new(0.0, 0.0, 0.0, 0.0));

                draw_sprites(&textures, args.sprites, elapsed as f32, world_visible);

                pop_camera_state();
            }

            // === STEP 3: Composite ===
            set_camera(&Camera2D {
                zoom: vec2(1.0 / screen_res.x * 2.0, 1.0 / screen_res.y * 2.0),
                target: vec2(screen_res.x / 2.0, screen_res.y / 2.0),
                ..Default::default()
            });

            if let Some(ref mat) = composite_material {
                // Set textures like wizard-pixels
                mat.set_texture("BackgroundTexture", background_rt.texture.weak_clone());
                mat.set_texture("AtomTexture", dummy_tex.clone());
                mat.set_texture("LightingTexture", dummy_tex.clone());
                mat.set_texture("BloomTexture", dummy_tex.clone());
                mat.set_texture("TerrainSdfTexture", dummy_tex.clone());
                mat.set_texture("LevelRawAtomsTexture", dummy_tex.clone());
                mat.set_texture("LevelRawRgbasTexture", dummy_tex.clone());
                mat.set_texture("SceneTexture", scene_rt.texture.weak_clone());
                gl_use_material(&mat);
                draw_texture_ex(
                    &dummy_tex,
                    0.0,
                    0.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(screen_res.x, screen_res.y)),
                        ..Default::default()
                    },
                );
                gl_use_default_material();
            } else {
                // Direct blit: draw background then scene
                draw_texture_ex(
                    &background_rt.texture,
                    0.0,
                    0.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(screen_res.x, screen_res.y)),
                        ..Default::default()
                    },
                );
                draw_texture_ex(
                    &scene_rt.texture,
                    0.0,
                    0.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(screen_res.x, screen_res.y)),
                        ..Default::default()
                    },
                );
            }
        } else {
            clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
            set_camera(&game_camera);

            draw_sprites(&textures, args.sprites, elapsed as f32, world_visible);
        }

        if frame_count == 10 {
            // Read back screen pixels to verify sprites are visible
            let img = get_screen_data();
            let mut nonzero = 0u32;
            for y in 0..img.height() {
                for x in 0..img.width() {
                    let px = img.get_pixel(x as u32, y as u32);
                    if px.r > 0.01 || px.g > 0.01 || px.b > 0.01 {
                        nonzero += 1;
                    }
                }
            }
            eprintln!("[VERIFY] Screen pixels with content: {} / {} ({}x{})",
                nonzero, img.width() * img.height(), img.width(), img.height());
            if args.fbo {
                let rt_img = scene_rt.texture.get_texture_data();
                let mut rt_nonzero = 0u32;
                for y in 0..rt_img.height() {
                    for x in 0..rt_img.width() {
                        let px = rt_img.get_pixel(x as u32, y as u32);
                        if px.a > 0.01 {
                            rt_nonzero += 1;
                        }
                    }
                }
                eprintln!("[VERIFY] scene_rt texture non-transparent: {} / {}",
                    rt_nonzero, rt_img.width() * rt_img.height());

                let bg_img = background_rt.texture.get_texture_data();
                let mut bg_nonzero = 0u32;
                for y in 0..bg_img.height() {
                    for x in 0..bg_img.width() {
                        let px = bg_img.get_pixel(x as u32, y as u32);
                        if px.a > 0.01 {
                            bg_nonzero += 1;
                        }
                    }
                }
                eprintln!("[VERIFY] bg_rt texture non-transparent: {} / {}",
                    bg_nonzero, bg_img.width() * bg_img.height());
            }
        }

        next_frame().await;
    }

    let total_time = get_time() - start_time;
    let avg_fps = frame_count as f64 / total_time;

    let mut sorted_times = frame_times.clone();
    sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let p50 = sorted_times[sorted_times.len() / 2];
    let p95 = sorted_times[(sorted_times.len() as f64 * 0.95) as usize];
    let p99 = sorted_times[(sorted_times.len() as f64 * 0.99) as usize];

    eprintln!("\n=== RESULTS ({}{}) ===",
        if args.fbo { "FBO" } else { "DEFAULT_FB" },
        if args.composite { "+COMPOSITE" } else { "" }
    );
    eprintln!("Total frames: {}", frame_count);
    eprintln!("Avg FPS: {:.1}", avg_fps);
    eprintln!("Frame time P50: {:.2}ms", p50 * 1000.0);
    eprintln!("Frame time P95: {:.2}ms", p95 * 1000.0);
    eprintln!("Frame time P99: {:.2}ms", p99 * 1000.0);
}

fn draw_sprites(textures: &[Texture2D], count: usize, time: f32, world_visible: Vec2) {
    let half_w = world_visible.x / 2.0;
    let half_h = world_visible.y / 2.0;

    for i in 0..count {
        let fi = i as f32;
        let angle = fi * 0.618033988 * std::f32::consts::TAU + time * 0.5;
        let radius = (fi * 0.01).sin().abs() * half_w.min(half_h) * 0.8;
        let x = angle.cos() * radius;
        let y = angle.sin() * radius;

        let tex_idx = i % textures.len();
        let size = 8.0 + (fi * 0.1).sin() * 4.0;

        draw_texture_ex(
            &textures[tex_idx],
            x - size / 2.0,
            y - size / 2.0,
            Color::new(1.0, 1.0, 1.0, 0.8 + (fi * 0.3).sin() * 0.2),
            DrawTextureParams {
                dest_size: Some(vec2(size, size)),
                ..Default::default()
            },
        );
    }
}
