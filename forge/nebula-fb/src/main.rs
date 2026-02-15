/// Nebula Framebuffer — AetherOS graphical shell.
/// Renders directly to /dev/fb0 at native resolution.

mod fb;
mod renderer;
mod text;
mod input;
mod theme;
mod scene;
mod layout;
mod brain_client;
mod telemetry;
mod widgets;
mod scenes;
mod audio;

use std::time::Instant;

fn main() {
    eprintln!("[nebula-fb] Starting AetherOS graphical shell");

    // Open framebuffer
    let mut framebuffer = match fb::Framebuffer::open("/dev/fb0") {
        Ok(fb) => fb,
        Err(e) => {
            eprintln!("[nebula-fb] Failed to open framebuffer: {}", e);
            eprintln!("[nebula-fb] Falling back to serial console nebula-tui if available");
            std::process::exit(1);
        }
    };

    let width = framebuffer.width();
    let height = framebuffer.height();
    eprintln!("[nebula-fb] Framebuffer: {}x{}", width, height);

    // Create renderer
    let mut render = renderer::Renderer::new(width, height);
    let text_renderer = text::TextRenderer::new();

    // Create input reader (with screen dimensions for mouse scaling)
    let mut input_reader = match input::InputReader::new_with_screen(width, height) {
        Ok(ir) => ir,
        Err(e) => {
            eprintln!("[nebula-fb] Warning: input init failed: {}", e);
            eprintln!("[nebula-fb] Continuing without keyboard input");
            std::process::exit(1);
        }
    };

    // Initialize audio player
    let audio_player = audio::AudioPlayer::new();

    // Initialize scene manager with boot splash
    let splash = scenes::boot_splash::BootSplash::new(width, height, &audio_player);
    let mut scene_manager = scene::SceneManager::new(Box::new(splash));

    // Main loop — idle at 10 FPS to save CPU, ramp up on input
    let frame_duration_active = std::time::Duration::from_millis(1000 / 30); // 30 FPS when active
    let frame_duration_idle = std::time::Duration::from_millis(1000 / 10);   // 10 FPS when idle
    let mut idle_frames: u32 = 0;

    eprintln!("[nebula-fb] Entering main loop at 30 FPS");

    let mut last_frame = Instant::now();

    loop {
        let now = Instant::now();
        let dt = now.duration_since(last_frame).as_secs_f32();
        last_frame = now;

        // Process input
        let event = input_reader.poll();
        let has_input = !matches!(event, input::InputEvent::None);
        if has_input {
            scene_manager.handle_input(event, &audio_player);
            idle_frames = 0;
        } else {
            idle_frames = idle_frames.saturating_add(1);
        }

        // Update
        scene_manager.update(dt, &audio_player);

        // Check if we should exit
        if scene_manager.is_empty() {
            break;
        }

        // Draw into renderer, then copy to back buffer
        scene_manager.draw(&mut render, &text_renderer);

        // Draw mouse cursor overlay
        draw_cursor(&mut render, input_reader.mouse_x, input_reader.mouse_y);

        render.copy_to(framebuffer.back_buffer_mut());

        // Only blit to framebuffer if pixels changed
        framebuffer.present();

        // Adaptive frame rate: fast when active, slow when idle
        let frame_duration = if idle_frames < 60 {
            frame_duration_active
        } else {
            frame_duration_idle
        };
        let elapsed = now.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    eprintln!("[nebula-fb] Exiting");
}

/// Draw a small arrow cursor at (mx, my).
/// 12x18 white arrow with 1px dark outline.
fn draw_cursor(render: &mut renderer::Renderer, mx: i32, my: i32) {
    // Arrow shape scanlines: (y_offset, x_start, x_end) for fill
    // Classic arrow pointer shape
    let outline = theme::Color::rgb(0x00, 0x00, 0x00);
    let fill = theme::Color::rgb(0xFF, 0xFF, 0xFF);

    // Arrow defined as rows: each row is (x_start, width) relative to cursor tip
    let arrow: &[(i32, i32)] = &[
        (0, 1),   // row 0: tip
        (0, 2),   // row 1
        (0, 3),   // row 2
        (0, 4),   // row 3
        (0, 5),   // row 4
        (0, 6),   // row 5
        (0, 7),   // row 6
        (0, 8),   // row 7
        (0, 9),   // row 8
        (0, 10),  // row 9
        (0, 11),  // row 10
        (0, 12),  // row 11: widest
        (0, 5),   // row 12: notch
        (2, 4),   // row 13
        (3, 4),   // row 14
        (4, 3),   // row 15
        (5, 3),   // row 16
        (6, 2),   // row 17
    ];

    let fx = mx as f32;
    let fy = my as f32;

    // Draw outline (1px border around each scanline)
    for (row_idx, &(x_off, w)) in arrow.iter().enumerate() {
        let y = fy + row_idx as f32;
        let x = fx + x_off as f32;
        // Top/bottom/left/right 1px outline
        render.fill_rect(x - 1.0, y, (w + 2) as f32, 1.0, outline);
        if row_idx == 0 || (row_idx > 0 && arrow[row_idx - 1].1 < w) {
            render.fill_rect(x, y - 0.5, w as f32, 1.0, outline);
        }
    }
    // Bottom outline for last row
    if let Some(&(x_off, w)) = arrow.last() {
        let y = fy + arrow.len() as f32;
        let x = fx + x_off as f32;
        render.fill_rect(x, y, w as f32, 1.0, outline);
    }

    // Draw white fill
    for (row_idx, &(x_off, w)) in arrow.iter().enumerate() {
        let y = fy + row_idx as f32;
        let x = fx + x_off as f32;
        render.fill_rect(x, y, w as f32, 1.0, fill);
    }
}
