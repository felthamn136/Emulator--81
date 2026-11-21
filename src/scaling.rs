use macroquad::prelude::*;

/// Maps logical design coordinates uniformly into the current window.
#[derive(Clone, Copy)]
pub struct Scaling {
    pub scale: f32,
    pub offset: Vec2,
}

impl Scaling {
    pub fn fit(bounds: Rect, design: Rect) -> Self {
        let scale = (bounds.w / design.w).min(bounds.h / design.h).max(0.0);
        Self {
            scale,
            offset: vec2(
                bounds.x + (bounds.w - design.w * scale) / 2.0 - design.x * scale,
                bounds.y + (bounds.h - design.h * scale) / 2.0 - design.y * scale,
            ),
        }
    }

    pub fn point(&self, x: f32, y: f32) -> Vec2 {
        self.offset + vec2(x, y) * self.scale
    }

    pub fn rect(&self, rect: Rect) -> Rect {
        let p = self.point(rect.x, rect.y);
        Rect::new(p.x, p.y, rect.w * self.scale, rect.h * self.scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_design_with_letterboxing() {
        let design = Rect::new(0.0, 0.0, 1600.0, 1200.0);
        let wide = Scaling::fit(Rect::new(0.0, 0.0, 1920.0, 1080.0), design);
        assert_eq!(wide.scale, 0.9);
        assert_eq!(wide.offset, vec2(240.0, 0.0));
        let tall = Scaling::fit(Rect::new(0.0, 0.0, 800.0, 1000.0), design);
        assert_eq!(tall.scale, 0.5);
        assert_eq!(tall.offset, vec2(0.0, 200.0));
    }

    #[test]
    fn supported_resolutions_preserve_the_crt_aspect_ratio() {
        for (width, height) in [
            (1280.0, 720.0),
            (1600.0, 1200.0),
            (1920.0, 1080.0),
            (2560.0, 1440.0),
            (3840.0, 2160.0),
            (800.0, 1200.0),
        ] {
            let bounds = crate::menu::content_rect(width, height);
            let scaling = Scaling::fit(bounds, crate::bezel::SCENE_BOUNDS);
            let viewport = crate::bezel::viewport_rect(&scaling);
            assert!((viewport.w / viewport.h - 4.0 / 3.0).abs() < 0.00001);
            let tv = scaling.rect(crate::bezel::SCENE_BOUNDS);
            assert!(tv.x >= bounds.x - 0.01 && tv.y >= bounds.y - 0.01);
            assert!(tv.x + tv.w <= bounds.x + bounds.w + 0.01);
            assert!(tv.y + tv.h <= bounds.y + bounds.h + 0.01);
            assert!(viewport.x >= bounds.x && viewport.y >= bounds.y);
            assert!(viewport.x + viewport.w <= bounds.x + bounds.w);
            assert!(viewport.y + viewport.h <= bounds.y + bounds.h);
        }
    }
}
