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

    // Create input reader
    let mut input_reader = match input::InputReader::new() {
        Ok(ir) => ir,
        Err(e) => {
            eprintln!("[nebula-fb] Warning: input init failed: {}", e);
            eprintln!("[nebula-fb] Continuing without keyboard input");
            // We'll handle this by making InputReader optional
            // For now, exit
            std::process::exit(1);
        }
    };

    // Initialize scene manager with boot splash
    let splash = scenes::boot_splash::BootSplash::new(width, height);
    let mut scene_manager = scene::SceneManager::new(Box::new(splash));

    // Main loop
    let mut last_frame = Instant::now();
    let target_fps = 30;
    let frame_duration = std::time::Duration::from_millis(1000 / target_fps);

    eprintln!("[nebula-fb] Entering main loop at {} FPS", target_fps);

    loop {
        let now = Instant::now();
        let dt = now.duration_since(last_frame).as_secs_f32();
        last_frame = now;

        // Process input
        let event = input_reader.poll();
        match event {
            input::InputEvent::None => {}
            _ => scene_manager.handle_input(event),
        }

        // Update
        scene_manager.update(dt);

        // Check if we should exit
        if scene_manager.is_empty() {
            break;
        }

        // Draw
        scene_manager.draw(&mut render, &text_renderer);

        // Blit to framebuffer
        render.copy_to(framebuffer.back_buffer_mut());
        framebuffer.present();

        // Frame rate limiting
        let elapsed = now.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    eprintln!("[nebula-fb] Exiting");
}
