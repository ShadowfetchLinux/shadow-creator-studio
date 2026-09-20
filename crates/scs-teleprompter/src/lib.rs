use serde::{Deserialize, Serialize};

pub const MIN_FONT: f32 = 18.0;
pub const MAX_FONT: f32 = 96.0;
pub const MIN_SPEED: f32 = 0.0;
pub const MAX_SPEED: f32 = 8.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeleprompterScript {
    pub title: String,
    pub body: String,
    pub scroll_speed: f32,
    pub font_size: f32,
    pub mirrored: bool,
    pub paused: bool,
}

impl Default for TeleprompterScript {
    fn default() -> Self {
        Self {
            title: String::new(),
            body: String::new(),
            scroll_speed: 1.0,
            font_size: 36.0,
            mirrored: false,
            paused: false,
        }
    }
}

impl TeleprompterScript {
    pub fn clamp(&mut self) {
        self.scroll_speed = clamp_speed(self.scroll_speed);
        self.font_size = clamp_font(self.font_size);
    }

    pub fn pixels_per_tick(&self, dt_secs: f32) -> f32 {
        if self.paused {
            0.0
        } else {
            clamp_speed(self.scroll_speed) * 28.0 * dt_secs.max(0.0)
        }
    }
}

pub fn clamp_font(size: f32) -> f32 {
    size.clamp(MIN_FONT, MAX_FONT)
}

pub fn clamp_speed(speed: f32) -> f32 {
    speed.clamp(MIN_SPEED, MAX_SPEED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_script_is_empty() {
        let script = TeleprompterScript::default();
        assert!(script.body.is_empty());
        assert_eq!(script.scroll_speed, 1.0);
    }

    #[test]
    fn font_and_speed_are_bounded() {
        assert_eq!(clamp_font(4.0), MIN_FONT);
        assert_eq!(clamp_font(400.0), MAX_FONT);
        assert_eq!(clamp_speed(-1.0), 0.0);
        assert_eq!(clamp_speed(99.0), MAX_SPEED);
        let mut script = TeleprompterScript::default();
        script.scroll_speed = 50.0;
        script.font_size = 2.0;
        script.clamp();
        assert_eq!(script.scroll_speed, MAX_SPEED);
        assert_eq!(script.font_size, MIN_FONT);
        script.paused = true;
        assert_eq!(script.pixels_per_tick(0.05), 0.0);
        script.paused = false;
        script.scroll_speed = 1.0;
        assert!(script.pixels_per_tick(0.05) > 0.0);
    }
}
