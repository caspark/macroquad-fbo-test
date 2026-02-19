use clap::Parser;
use macroquad::prelude::*;

#[derive(Parser)]
struct Args {
    /// Use scene render target (FBO) instead of default framebuffer
    #[arg(long)]
    fbo: bool,

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

#[macroquad::main(window_conf)]
async fn main() {
    let args = Args::parse();
    let screen_res = vec2(screen_width(), screen_height());

    eprintln!(
        "Mode: {}, sprites: {}, textures: {}, scale: {}, screen: {}x{}",
        if args.fbo { "FBO" } else { "DEFAULT_FB" },
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

    let scene_rt = render_target(screen_res.x as u32, screen_res.y as u32);
    scene_rt.texture.set_filter(FilterMode::Nearest);

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
        } else {
            clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
            set_camera(&game_camera);

            draw_sprites(&textures, args.sprites, elapsed as f32, world_visible);
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

    eprintln!("\n=== RESULTS ({}) ===", if args.fbo { "FBO" } else { "DEFAULT_FB" });
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

        // Cycle through textures to break batching
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
