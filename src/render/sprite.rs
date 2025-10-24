use image::{ImageBuffer, Rgba};
use wgpu::{Extent3d, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture};

use super::{Displayable, Gpu, Images};

use crate::*;

#[derive(Clone)]
pub struct PalleteSwap {
    pub from: Vec<Rgba<u8>>,
    pub to: Vec<Rgba<u8>>,
}

impl PalleteSwap {
    pub fn new(from: Vec<Rgba<u8>>, to: Vec<Rgba<u8>>) -> Self {
        assert_eq!(from.len(), to.len());
        Self { from, to }
    }

    fn parse_color(s: &str) -> Option<Rgba<u8>> {
        let s = s.trim_start_matches('#');
        if s.len() != 6 && s.len() != 8 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        let a = if s.len() == 8 {
            u8::from_str_radix(&s[6..8], 16).ok()?
        } else {
            255
        };
        Some(Rgba([r, g, b, a]))
    }

    pub fn load(contents: &str) -> Self {
        let mut from = Vec::new();
        let mut to = Vec::new();

        for line in contents.lines() {
            if line.trim().is_empty() || line.trim_start().starts_with("//") {
                continue;
            }

            let parts: Vec<&str> = line.split("->").map(|s| s.trim()).collect();
            if parts.len() != 2 {
                eprintln!("Invalid pallete swap line: {}", line);
                continue;
            }

            if let (Some(f), Some(t)) = (Self::parse_color(parts[0]), Self::parse_color(parts[1])) {
                from.push(f);
                to.push(t);
            } else {
                eprintln!("Invalid color in pallete swap: {}", line);
            }
        }

        Self { from, to }
    }

    pub fn apply(&self, image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
        for pixel in image.pixels_mut() {
            for (i, from_color) in self.from.iter().enumerate() {
                if pixel == from_color {
                    *pixel = self.to[i];
                    break;
                }
            }
        }
    }
}

#[derive(Component)]
pub struct Sprite {
    pub h: u32,
    pub w: u32,
    pub tex: Texture,
}

impl Displayable for Sprite {
    fn get_texture_and_size(&self) -> (&Texture, Extent3d) {
        (
            &self.tex,
            Extent3d {
                width: self.w,
                height: self.h,
                depth_or_array_layers: 1,
            },
        )
    }
}

pub struct SpriteBuilder {
    pub h: u32,
    pub w: u32,
    pub x: u32,
    pub y: u32,

    pub image_path: String,
    pub pallete_swap: Option<PalleteSwap>,
}

impl Default for SpriteBuilder {
    fn default() -> Self {
        Self {
            h: 0,
            w: 0,
            x: 0,
            y: 0,

            image_path: "rawr".to_string(),
            pallete_swap: None,
        }
    }
}

impl SpriteBuilder {
    pub fn build(&self, gpu: &Gpu, images: &Images) -> Sprite {
        let img = images
            .images
            .get(&self.image_path)
            .expect("Failed to load image");
        let mut img = img.clone();
        if self.w != 0 && self.h != 0 {
            img = image::imageops::crop_imm(&img, self.x, self.y, self.w, self.h).to_image();
        }
        if let Some(pallete_swap) = &self.pallete_swap {
            pallete_swap.apply(&mut img);
        }

        let size = wgpu::Extent3d {
            width: img.width(),
            height: img.height(),
            depth_or_array_layers: 1,
        };

        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Sprite Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let w = img.width();
        let h = img.height();

        gpu.queue.write_texture(
            TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            img.into_raw().as_slice(),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.w),
                rows_per_image: Some(self.h),
            },
            Extent3d {
                width: self.w,
                height: self.h,
                depth_or_array_layers: 1,
            },
        );

        Sprite { h, w, tex }
    }
}

#[derive(Component)]
pub struct Animation {
    pub frames: Vec<Sprite>,
    pub time_between_frames: f32,
    pub time_accumulator: f32,
    pub current_frame: usize,

    pub looping: bool,
    pub running: bool,
}

impl Displayable for Animation {
    fn get_texture_and_size(&self) -> (&Texture, Extent3d) {
        self.current_sprite().get_texture_and_size()
    }
}

impl Animation {
    pub fn from_frames(frames: Vec<Sprite>, speed: f32, looping: bool, running: bool) -> Self {
        Self {
            frames,
            time_between_frames: if speed == 0.0 { f32::MAX } else { 1.0 / speed },
            time_accumulator: 0.0,
            current_frame: 0,
            looping,
            running,
        }
    }

    pub fn from_spritesheet(
        path: String,
        gpu: &Gpu,
        images: &Images,
        pallete_swap: Option<PalleteSwap>,
        frame_w: u32,
        frame_h: u32,
        speed: f32,
        looping: bool,
        running: bool,
    ) -> Self {
        let img = images
            .images
            .get(&path)
            .expect("Failed to load spritesheet");

        let (sheet_w, sheet_h) = img.dimensions();
        let cols = sheet_w / frame_w;
        let rows = sheet_h / frame_h;

        let mut frames = Vec::new();
        for y in 0..rows {
            for x in 0..cols {
                let sprite = SpriteBuilder {
                    h: frame_h,
                    w: frame_w,
                    x: x * frame_w,
                    y: y * frame_h,
                    image_path: path.clone(),
                    pallete_swap: pallete_swap.clone(),
                }
                .build(gpu, images);

                frames.push(sprite);
            }
        }

        Self {
            frames,
            time_between_frames: if speed == 0.0 { f32::MAX } else { 1.0 / speed },
            time_accumulator: 0.0,
            current_frame: 0,
            looping,
            running,
        }
    }

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn update(&mut self, delta_time: f32) {
        if !self.running {
            return;
        }
        self.time_accumulator += delta_time;
        while self.time_accumulator >= self.time_between_frames {
            if !self.looping && self.current_frame == self.frames.len() - 1 {
                self.running = false;
                return;
            }
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            self.time_accumulator -= self.time_between_frames;
        }
    }

    pub fn current_sprite(&self) -> &Sprite {
        &self.frames[self.current_frame]
    }

    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.time_accumulator = 0.0;
    }

    pub fn advance(&mut self) {
        if !self.looping && self.current_frame == self.frames.len() - 1 {
            return;
        }
        self.current_frame = (self.current_frame + 1) % self.frames.len();
    }

    pub fn retreat(&mut self) {
        if self.current_frame == 0 {
            if !self.looping {
                return;
            }
            self.current_frame = self.frames.len() - 1;
        } else {
            self.current_frame -= 1;
        }
    }

    pub fn is_finished(&self) -> bool {
        !self.looping && self.current_frame == self.frames.len() - 1
    }
}

system!(
    fn update_animations(
        time: res &Time,
        anims: query (&mut Animation),
    ) {
        let Some(time) = time else {
            return;
        };

        for anim in anims {
            anim.update(time.delta_seconds);
        }
    }
);
