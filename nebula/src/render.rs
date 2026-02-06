//! GPU Renderer
//!
//! Abstracts over wgpu to provide a simple 2D rendering API.
//! Uses a CPU-side pixel buffer for 2D primitives, uploaded as a texture each frame.
//! In development: uses winit window
//! In production: uses DRM/KMS directly

use anyhow::Result;
use glam::Vec2;
use std::sync::Arc;
use tracing::info;

/// Colors with alpha
#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    // Aether color palette
    pub const VOID: Self = Self::rgb(0.02, 0.02, 0.04);
    pub const SURFACE: Self = Self::rgb(0.08, 0.08, 0.12);
    pub const TEXT: Self = Self::rgb(0.9, 0.9, 0.92);
    pub const TEXT_DIM: Self = Self::rgba(0.9, 0.9, 0.92, 0.5);
    pub const ACCENT: Self = Self::rgb(0.4, 0.6, 1.0);
    pub const GLOW: Self = Self::rgba(0.4, 0.6, 1.0, 0.3);

    fn to_rgba8(&self) -> [u8; 4] {
        [
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
            (self.a.clamp(0.0, 1.0) * 255.0) as u8,
        ]
    }
}

/// A rectangle for rendering
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn centered(center: Vec2, width: f32, height: f32) -> Self {
        Self {
            x: center.x - width / 2.0,
            y: center.y - height / 2.0,
            width,
            height,
        }
    }
}

/// Render commands that accumulate during a frame
#[derive(Clone, Debug)]
pub enum RenderCommand {
    Clear(Color),
    Rect {
        rect: Rect,
        color: Color,
        corner_radius: f32,
    },
    Text {
        text: String,
        position: Vec2,
        size: f32,
        color: Color,
    },
    Blur {
        rect: Rect,
        radius: f32,
    },
}

/// The renderer
pub struct Renderer {
    width: u32,
    height: u32,
    commands: Vec<RenderCommand>,
    pixels: Vec<u8>,

    // wgpu state
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    texture: wgpu::Texture,
    texture_bind_group: wgpu::BindGroup,
}

impl Renderer {
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self> {
        info!("Initializing wgpu renderer");

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| anyhow::anyhow!("Failed to find a suitable GPU adapter"))?;

        info!("Using GPU: {}", adapter.get_info().name);

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Nebula Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Create the pixel buffer texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Framebuffer Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Framebuffer Sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fullscreen Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blit Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blit Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let pixel_count = (width * height * 4) as usize;
        let pixels = vec![0u8; pixel_count];

        info!("Renderer initialized: {}x{}", width, height);

        Ok(Self {
            width,
            height,
            commands: Vec::new(),
            pixels,
            device,
            queue,
            surface,
            surface_config,
            render_pipeline,
            texture,
            texture_bind_group,
        })
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        let new_width = new_width.max(1);
        let new_height = new_height.max(1);
        if new_width == self.width && new_height == self.height {
            return;
        }

        self.width = new_width;
        self.height = new_height;
        self.surface_config.width = new_width;
        self.surface_config.height = new_height;
        self.surface.configure(&self.device, &self.surface_config);

        // Recreate pixel buffer and texture
        self.pixels = vec![0u8; (new_width * new_height * 4) as usize];

        self.texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Framebuffer Texture"),
            size: wgpu::Extent3d {
                width: new_width,
                height: new_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = self.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Framebuffer Sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = self.render_pipeline.get_bind_group_layout(0);
        self.texture_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(self.width as f32 / 2.0, self.height as f32 / 2.0)
    }

    pub fn begin_frame(&mut self) {
        self.commands.clear();
        self.commands.push(RenderCommand::Clear(Color::VOID));
    }

    pub fn draw_rect(&mut self, rect: Rect, color: Color, corner_radius: f32) {
        self.commands.push(RenderCommand::Rect {
            rect,
            color,
            corner_radius,
        });
    }

    pub fn draw_text(&mut self, text: &str, position: Vec2, size: f32, color: Color) {
        self.commands.push(RenderCommand::Text {
            text: text.to_string(),
            position,
            size,
            color,
        });
    }

    pub fn draw_blur(&mut self, rect: Rect, radius: f32) {
        self.commands.push(RenderCommand::Blur { rect, radius });
    }

    pub fn end_frame(&mut self) -> Result<()> {
        // Rasterize commands to pixel buffer
        let commands = self.commands.clone();
        for cmd in &commands {
            match cmd {
                RenderCommand::Clear(color) => {
                    self.raster_clear(color);
                }
                RenderCommand::Rect { rect, color, corner_radius } => {
                    self.raster_rect(rect, color, *corner_radius);
                }
                RenderCommand::Text { text, position, size, color } => {
                    self.raster_text(text, *position, *size, color);
                }
                RenderCommand::Blur { .. } => {
                    // Blur is a no-op for now (would need multi-pass)
                }
            }
        }

        // Upload pixel buffer to GPU texture
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.width),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        // Render the texture to screen
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Blit Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blit Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    // --- Software rasterization ---

    fn set_pixel(&mut self, x: i32, y: i32, color: &Color) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = ((y as u32 * self.width + x as u32) * 4) as usize;
        if idx + 3 >= self.pixels.len() {
            return;
        }

        let src = color.to_rgba8();
        let sa = src[3] as f32 / 255.0;

        if sa >= 1.0 {
            self.pixels[idx] = src[0];
            self.pixels[idx + 1] = src[1];
            self.pixels[idx + 2] = src[2];
            self.pixels[idx + 3] = 255;
        } else if sa > 0.0 {
            // Alpha blend
            let da = 1.0 - sa;
            self.pixels[idx] = (src[0] as f32 * sa + self.pixels[idx] as f32 * da) as u8;
            self.pixels[idx + 1] = (src[1] as f32 * sa + self.pixels[idx + 1] as f32 * da) as u8;
            self.pixels[idx + 2] = (src[2] as f32 * sa + self.pixels[idx + 2] as f32 * da) as u8;
            self.pixels[idx + 3] = ((sa + self.pixels[idx + 3] as f32 / 255.0 * da) * 255.0) as u8;
        }
    }

    fn raster_clear(&mut self, color: &Color) {
        let rgba = color.to_rgba8();
        for chunk in self.pixels.chunks_exact_mut(4) {
            chunk[0] = rgba[0];
            chunk[1] = rgba[1];
            chunk[2] = rgba[2];
            chunk[3] = rgba[3];
        }
    }

    fn raster_rect(&mut self, rect: &Rect, color: &Color, corner_radius: f32) {
        let x0 = rect.x as i32;
        let y0 = rect.y as i32;
        let x1 = (rect.x + rect.width) as i32;
        let y1 = (rect.y + rect.height) as i32;
        let cr = corner_radius.min(rect.width / 2.0).min(rect.height / 2.0);

        for py in y0..y1 {
            for px in x0..x1 {
                if cr > 0.5 {
                    // Check if pixel is within rounded corners
                    let lx = px as f32 - rect.x;
                    let ly = py as f32 - rect.y;
                    let rx = rect.width - lx;
                    let ry = rect.height - ly;

                    let in_corner = if lx < cr && ly < cr {
                        let dx = cr - lx;
                        let dy = cr - ly;
                        dx * dx + dy * dy <= cr * cr
                    } else if rx < cr && ly < cr {
                        let dx = cr - rx;
                        let dy = cr - ly;
                        dx * dx + dy * dy <= cr * cr
                    } else if lx < cr && ry < cr {
                        let dx = cr - lx;
                        let dy = cr - ry;
                        dx * dx + dy * dy <= cr * cr
                    } else if rx < cr && ry < cr {
                        let dx = cr - rx;
                        let dy = cr - ry;
                        dx * dx + dy * dy <= cr * cr
                    } else {
                        true
                    };

                    if in_corner {
                        self.set_pixel(px, py, color);
                    }
                } else {
                    self.set_pixel(px, py, color);
                }
            }
        }
    }

    fn raster_text(&mut self, text: &str, position: Vec2, size: f32, color: &Color) {
        // Simple bitmap font rendering -- each glyph is a 5x7 pixel grid scaled to `size`
        let scale = (size / 10.0).max(0.5);
        let glyph_w = (6.0 * scale) as i32;
        let mut cx = position.x as i32;
        let cy = position.y as i32;

        for ch in text.chars() {
            if let Some(bitmap) = get_glyph(ch) {
                for row in 0..7 {
                    for col in 0..5 {
                        if bitmap[row] & (1 << (4 - col)) != 0 {
                            // Scale the pixel
                            let px_base = cx + (col as f32 * scale) as i32;
                            let py_base = cy + (row as f32 * scale) as i32;
                            let px_end = cx + ((col + 1) as f32 * scale) as i32;
                            let py_end = cy + ((row + 1) as f32 * scale) as i32;
                            for py in py_base..py_end {
                                for px in px_base..px_end {
                                    self.set_pixel(px, py, color);
                                }
                            }
                        }
                    }
                }
            }
            cx += glyph_w;
        }
    }
}

/// Minimal 5x7 bitmap font for basic ASCII
fn get_glyph(c: char) -> Option<[u8; 7]> {
    Some(match c {
        ' ' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '"' => [0b01010, 0b01010, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        '#' => [0b01010, 0b11111, 0b01010, 0b01010, 0b11111, 0b01010, 0b00000],
        '$' => [0b00100, 0b01111, 0b10100, 0b01110, 0b00101, 0b11110, 0b00100],
        '%' => [0b11001, 0b11010, 0b00100, 0b00100, 0b01011, 0b10011, 0b00000],
        '&' => [0b01100, 0b10010, 0b01100, 0b10101, 0b10010, 0b01101, 0b00000],
        '\'' => [0b00100, 0b00100, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        '(' => [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
        ')' => [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
        '*' => [0b00000, 0b00100, 0b10101, 0b01110, 0b10101, 0b00100, 0b00000],
        '+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
        ',' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b01000],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100],
        '/' => [0b00001, 0b00010, 0b00100, 0b00100, 0b01000, 0b10000, 0b00000],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        '3' => [0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
        ':' => [0b00000, 0b00000, 0b00100, 0b00000, 0b00000, 0b00100, 0b00000],
        ';' => [0b00000, 0b00000, 0b00100, 0b00000, 0b00000, 0b00100, 0b01000],
        '<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
        '=' => [0b00000, 0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000],
        '>' => [0b10000, 0b01000, 0b00100, 0b00010, 0b00100, 0b01000, 0b10000],
        '?' => [0b01110, 0b10001, 0b00001, 0b00110, 0b00100, 0b00000, 0b00100],
        '@' => [0b01110, 0b10001, 0b10111, 0b10101, 0b10110, 0b10000, 0b01110],
        'A' | 'a' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' | 'b' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' | 'c' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' | 'd' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' | 'e' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' | 'f' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' | 'g' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        'H' | 'h' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' | 'i' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' | 'j' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' | 'k' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' | 'l' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' | 'm' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' | 'n' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' | 'o' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' | 'p' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' | 'q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' | 'r' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' | 's' => [0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110],
        'T' | 't' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' | 'u' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' | 'v' => [0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b01010, 0b00100],
        'W' | 'w' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' | 'x' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' | 'y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' | 'z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '[' => [0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110],
        ']' => [0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110],
        '_' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111],
        _ => return None,
    })
}

const BLIT_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle pair
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );

    var out: VertexOutput;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv);
}
"#;
