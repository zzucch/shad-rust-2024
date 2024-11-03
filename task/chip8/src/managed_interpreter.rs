use crate::{
    data::Word,
    error::Result,
    image::Image,
    interpreter::{Interpreter, SCREEN_HEIGHT, SCREEN_WIDTH},
    platform::{Key, Platform, Point, Sprite},
    Error, KeyEventKind, Nibble,
};

use core::time::Duration;

////////////////////////////////////////////////////////////////////////////////

pub const KEYPAD_SIZE: usize = 16;

pub struct FrameBuffer([[bool; SCREEN_WIDTH]; SCREEN_HEIGHT]);

impl Default for FrameBuffer {
    fn default() -> Self {
        Self([[false; SCREEN_WIDTH]; SCREEN_HEIGHT])
    }
}

impl FrameBuffer {
    pub fn iter_rows(&self) -> impl Iterator<Item = &[bool; SCREEN_WIDTH]> {
        self.0.iter()
    }

    pub fn clear(&mut self) {
        for row in self.0.iter_mut() {
            for element in row.iter_mut() {
                *element = false
            }
        }
    }

    pub fn flip(&mut self, point: Point, start: Point) -> bool {
        let target = start + point;

        let y = target.y as usize;
        let x = target.x as usize;

        if y >= SCREEN_HEIGHT || x >= SCREEN_WIDTH {
            return false;
        }

        let previous_value = self.0[y][x];
        self.0[y][x] = !previous_value;

        previous_value
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait RandomNumberGenerator: FnMut() -> Word {}

impl<R: FnMut() -> Word> RandomNumberGenerator for R {}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
struct ManagedPlatform<R: RandomNumberGenerator> {
    rand: R,
    frame_buffer: FrameBuffer,
    delay_timer: Word,
    sound_timer: Word,
    keypad: ManagedKeypad,
}

impl<R: RandomNumberGenerator> Platform for ManagedPlatform<R> {
    fn draw_sprite(&mut self, pos: Point, sprite: Sprite) -> bool {
        let wrapped_pos = wrap_point_within_screen(pos);

        let mut had_pixels_flipped = false;
        for pixel in sprite.iter_pixels() {
            had_pixels_flipped |= self.frame_buffer.flip(pixel, wrapped_pos);
        }

        had_pixels_flipped
    }

    fn clear_screen(&mut self) {
        self.frame_buffer.clear()
    }

    fn get_delay_timer(&self) -> Word {
        self.delay_timer
    }

    fn set_delay_timer(&mut self, value: Word) {
        self.delay_timer = value
    }

    fn set_sound_timer(&mut self, value: Word) {
        self.sound_timer = value
    }

    fn is_key_down(&self, key: Key) -> bool {
        self.keypad.get_key_kind(key).expect("key must be valid") == KeyEventKind::Pressed
    }

    fn consume_key_press(&mut self) -> Option<Key> {
        self.keypad.consume_key_press()
    }

    fn get_random_word(&mut self) -> Word {
        (self.rand)()
    }
}

fn wrap_point_within_screen(point: Point) -> Point {
    Point {
        x: (point.x as usize % SCREEN_WIDTH) as u8,
        y: (point.y as usize % SCREEN_HEIGHT) as u8,
    }
}

impl<R: RandomNumberGenerator> ManagedPlatform<R> {
    fn new(rand: R) -> Self {
        Self {
            rand,
            frame_buffer: Default::default(),
            delay_timer: 0,
            sound_timer: 0,
            keypad: ManagedKeypad::default(),
        }
    }
}

struct ManagedKeypad {
    keys: [KeyEventKind; KEYPAD_SIZE],
    last_pressed_key: Option<Key>,
}

impl Default for ManagedKeypad {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedKeypad {
    fn new() -> Self {
        Self {
            keys: [KeyEventKind::Released; KEYPAD_SIZE],
            last_pressed_key: None,
        }
    }

    fn set_key(&mut self, key: Key, event_kind: KeyEventKind) -> Result<()> {
        if key.as_usize() >= self.keys.len() {
            return Err(Error::InvalidKey(key.as_u8()));
        }

        match event_kind {
            KeyEventKind::Pressed => {
                self.keys[key.as_usize()] = KeyEventKind::Pressed;

                self.last_pressed_key = Some(key);
            }
            KeyEventKind::Released => {
                self.keys[key.as_usize()] = KeyEventKind::Released;
            }
        }

        Ok(())
    }

    fn get_key_kind(&self, key: Nibble) -> Result<KeyEventKind> {
        if key.as_usize() >= self.keys.len() {
            return Err(Error::InvalidKey(key.as_u8()));
        }

        Ok(self.keys[key.as_usize()])
    }

    fn consume_key_press(&mut self) -> Option<Nibble> {
        if let Some(last) = self.last_pressed_key {
            if self.keys[last.as_usize()] == KeyEventKind::Released {
                self.last_pressed_key = None;

                return Some(last);
            }
        }

        None
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct ManagedInterpreter<R: RandomNumberGenerator> {
    inner: Interpreter<ManagedPlatform<R>>,
    operation_duration: Duration,
    delay_tick_duration: Duration,
    sound_tick_duration: Duration,
}

impl<R: RandomNumberGenerator> ManagedInterpreter<R> {
    pub const DEFAULT_OPERATION_DURATION: Duration = Duration::from_millis(2);
    pub const DEFAULT_DELAY_TICK_DURATION: Duration = Duration::from_nanos(16666667);
    pub const DEFAULT_SOUND_TICK_DURATION: Duration = Duration::from_nanos(16666667);

    pub fn new(image: impl Image, rand: R) -> Self {
        Self::new_with_durations(
            image,
            rand,
            Self::DEFAULT_OPERATION_DURATION,
            Self::DEFAULT_DELAY_TICK_DURATION,
            Self::DEFAULT_SOUND_TICK_DURATION,
        )
    }

    pub fn new_with_durations(
        image: impl Image,
        rand: R,
        operation_duration: Duration,
        delay_tick_duration: Duration,
        sound_tick_duration: Duration,
    ) -> Self {
        Self {
            inner: Interpreter::new(image, ManagedPlatform::new(rand)),
            operation_duration,
            delay_tick_duration,
            sound_tick_duration,
        }
    }

    pub fn simulate_one_instruction(&mut self) -> Result<()> {
        self.inner.run_next_instruction()
    }

    pub fn simulate_duration(&mut self, duration: Duration) -> Result<()> {
        for millisecond in 0..duration.as_millis() {
            if millisecond % self.operation_duration.as_millis() == 0 {
                self.inner.run_next_instruction()?
            }

            if millisecond % self.delay_tick_duration.as_millis() == 0 {
                let delay_timer_value = self
                    .inner
                    .platform_mut()
                    .get_delay_timer()
                    .saturating_sub(1);

                self.inner.platform_mut().set_delay_timer(delay_timer_value);
            }

            if millisecond % self.sound_tick_duration.as_millis() == 0 {
                let sound_timer_value = self.inner.platform_mut().sound_timer.saturating_sub(1);

                self.inner.platform_mut().set_sound_timer(sound_timer_value);
            }
        }
        Ok(())
    }

    pub fn frame_buffer(&self) -> &FrameBuffer {
        &self.inner.platform().frame_buffer
    }

    pub fn set_key_down(&mut self, key: Key, is_down: bool) {
        let event_kind = if is_down {
            KeyEventKind::Pressed
        } else {
            KeyEventKind::Released
        };

        self.inner
            .platform_mut()
            .keypad
            .set_key(key, event_kind)
            .expect("key must be valid");
    }
}
