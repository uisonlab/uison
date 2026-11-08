//! One frame: wait for the GPU, paint the regions into a `Scene` through `VkCanvas`, submit.

use uison_core::Rect;

use crate::canvas::{Res, VkCanvas};
use crate::ffi::{GetModuleHandleW, HWND, RECT};
use crate::gpu::{Gpu, Scene};
use crate::text::Text;

pub struct Renderer {
    gpu: Gpu,
    scene: Scene,
    res: Res,
}

impl Renderer {
    pub fn new(hwnd: HWND) -> Result<Renderer, String> {
        let gpu = Gpu::new(hwnd, unsafe { GetModuleHandleW(std::ptr::null()) })?;
        Ok(Renderer { gpu, scene: Scene::default(), res: Res::new()? })
    }

    pub fn device_name(&self) -> &str {
        &self.gpu.device_name
    }

    pub fn resized(&mut self) {
        self.gpu.resized();
    }

    /// Draw the regions `rects` (window pixels); `paint` paints one region (points). When the
    /// window's pixels were lost (first frame, new size), the whole window is painted.
    /// Returns (regions painted, whether the window should be drawn again); `Err` — the device is lost.
    pub fn draw(&mut self, text: &Text, rects: &[RECT], scale: f32, mut paint: impl FnMut(&mut VkCanvas, Rect)) -> Result<(usize, bool), String> {
        let Some((extent, fresh)) = self.gpu.begin()? else { return Ok((0, false)) };
        // The previous frame is done: textures it could still draw are free to go.
        for p in std::mem::take(&mut self.res.retired_pipelines) {
            self.gpu.free_pipeline(p);
        }
        for t in std::mem::take(&mut self.res.retired) {
            self.gpu.free_texture(t);
        }
        let (w, h) = (extent.width as i32, extent.height as i32);
        // The atlas outlives the target: text drawn before a resize is reused after it.
        self.res.bounds = [0, 0, w, h];
        let full = [RECT { left: 0, top: 0, right: w, bottom: h }];
        let rects = if fresh { &full[..] } else { rects };
        let regions: Vec<Rect> = rects
            .iter()
            .map(|r| Rect::new(r.left as f32 / scale, r.top as f32 / scale, (r.right - r.left) as f32 / scale, (r.bottom - r.top) as f32 / scale))
            .collect();
        self.scene.clear();
        for attempt in 0..2 {
            self.res.overflow = false;
            let mut canvas = VkCanvas::new(&mut self.gpu, &mut self.scene, &mut self.res, text, scale);
            for &r in &regions {
                paint(&mut canvas, r);
            }
            if !self.res.overflow || attempt == 1 {
                break;
            }
            // The atlas is full: start it over and paint the regions again (images keep their uploads).
            self.res.reset_atlas();
            self.scene.restart();
        }
        let again = self.gpu.submit(&self.scene)?;
        Ok((regions.len(), again))
    }
}
