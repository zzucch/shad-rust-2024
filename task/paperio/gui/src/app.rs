use std::{
    collections::HashMap,
    future::Future,
    io::{BufRead, Write},
    ops::DerefMut,
    sync::{
        atomic::{AtomicU64, AtomicU8, Ordering},
        Arc, Mutex,
    },
};

use crate::{
    colors::{cell_color, colors_for_player, head_color},
    state::GameState,
};

use anyhow::bail;
use eframe::egui;
use egui::{pos2, vec2, Align, Color32, Layout, Rect, RichText, Sense, Slider, Vec2};
use num_traits::FromPrimitive;
use paperio_proto::{
    traits::{JsonRead, JsonWrite},
    Cell, Command, Direction, Message, PlayerId, PlayerInfo,
};

const KEY_MAP: [(egui::Key, Direction); 4] = [
    (egui::Key::ArrowUp, Direction::Up),
    (egui::Key::ArrowDown, Direction::Down),
    (egui::Key::ArrowRight, Direction::Right),
    (egui::Key::ArrowLeft, Direction::Left),
];

enum State {
    AwaitForGameStart,
    Tick(GameState),
    Ended,
}

pub struct PaperioApp {
    state: Arc<Mutex<State>>,
    direction: AtomicDirection,
    tick_duration: Arc<AtomicU64>,
    is_spectator: bool,
    player_nicknames: Option<HashMap<PlayerId, PlayerInfo>>,
}

impl PaperioApp {
    pub fn new(tick_delay_ms: u64, is_spectator: bool) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self {
            state: Arc::new(Mutex::new(State::AwaitForGameStart)),
            direction: AtomicDirection::new(Direction::Left),
            tick_duration: Arc::new(AtomicU64::new(tick_delay_ms)),
            is_spectator,
            player_nicknames: None,
        }
    }

    pub fn set_nicknames(&mut self, nicknames: HashMap<PlayerId, PlayerInfo>) {
        self.player_nicknames = Some(nicknames)
    }

    fn get_nickname(&self, player_id: &PlayerId) -> String {
        self.player_nicknames
            .as_ref()
            .and_then(|nicknames| nicknames.get(player_id).map(|i| &i.user_name).cloned())
            .unwrap_or_else(|| {
                if player_id == "i" {
                    "Me".to_string()
                } else {
                    format!("Player #{player_id}")
                }
            })
    }
}

impl PaperioApp {
    pub fn run_backend(
        &self,
        mut reader: impl BufRead + Send + 'static,
        mut writer: impl Write + Send + 'static,
    ) -> impl Future<Output = anyhow::Result<()>> {
        let state = self.state.clone();
        let direction_store = self.direction.clone();
        let tick_duration_store = self.tick_duration.clone();
        let is_spectator = self.is_spectator;

        async move {
            // receive `GameParams` msg
            log::info!("Waiting for the first message from server with game params");
            let Message::StartGame(params) = reader.read_message()? else {
                bail!("first message is not `StartGame`")
            };
            *state.lock().unwrap() = State::Tick(GameState::new(params));

            // receive tick msgs
            log::info!("Entering loop of receiving tick messages");
            loop {
                let read_message = reader.read_message()?;
                match read_message {
                    Message::StartGame(_) => bail!("unexpected `StartGame` message"),
                    Message::Tick(world) => {
                        let mut state_guard = state.lock().unwrap();
                        match state_guard.deref_mut() {
                            State::AwaitForGameStart => {
                                bail!("unexpected tick while waiting for game to start")
                            }
                            State::Tick(game_field) => {
                                game_field.update(world);
                            }
                            State::Ended => bail!("unexpected tick when game ended"),
                        }
                    }
                    Message::EndGame {} => {
                        log::info!("End game message received");
                        *state.lock().unwrap() = State::Ended;
                        break;
                    }
                }

                let tick_ms = tick_duration_store.load(Ordering::Relaxed);

                #[cfg(not(target_arch = "wasm32"))]
                {
                    std::thread::sleep(std::time::Duration::from_millis(tick_ms));
                }
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::TimeoutFuture::new(tick_ms as u32).await;

                let cmd = if is_spectator {
                    Command::NoOp
                } else {
                    let direction = direction_store.load();
                    Command::ChangeDirection(direction)
                };
                writer.write_command(&cmd)?;
                writer.flush()?;
            }
            Ok(())
        }
    }

    fn draw_field(&self, ui: &mut egui::Ui, game: &GameState) {
        let params = game.params;
        let size_in_cells = vec2(params.x_cells_count as f32, params.y_cells_count as f32);
        let size_in_pixels = ui.available_size_before_wrap();
        let cell_size_with_border = (size_in_pixels / size_in_cells).floor().min_elem();
        let cell_sizes = Vec2::splat(cell_size_with_border - 1.);

        let (_, painter) =
            ui.allocate_painter(size_in_cells * cell_size_with_border, Sense::hover());

        let zero_pos = ui.min_rect().min.to_vec2();
        let draw_cell = |Cell(x, y): Cell, color: Color32| {
            // Game indexation is down-to-top, but we draw top-to-down, so invert Oy here.
            let y = params.y_cells_count - 1 - y as u32;
            let rect_corner = pos2(x as f32, y as f32) * cell_size_with_border + zero_pos;
            let rect = Rect::from_min_size(rect_corner, cell_sizes);
            painter.rect_filled(rect, 0., color);
        };

        for (y, row) in game.field.iter().enumerate() {
            for (x, c) in row.iter().enumerate() {
                let color = cell_color(c);
                draw_cell(Cell(x as i32, y as i32), color)
            }
        }
        for (id, player) in &game.world.players {
            if !player.has_lost {
                let color = head_color(id);
                draw_cell(player.position, color)
            }
        }
    }
}

impl eframe::App for PaperioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut state_guard = self.state.lock().unwrap();
            match state_guard.deref_mut() {
                State::AwaitForGameStart => {
                    ui.label("Waiting to 'start_game'");
                }
                State::Tick(ref game) => {
                    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                        self.draw_field(ui, game);

                        ui.with_layout(Layout::top_down(Align::Min), |ui| {
                            let mut scores = game
                                .world
                                .players
                                .iter()
                                .map(|(id, p)| (id, p.score))
                                .collect::<Vec<_>>();

                            scores.sort_unstable_by(|(id1, s1), (id2, s2)| {
                                s2.cmp(s1).then(id1.cmp(id2))
                            });

                            for (id, score) in &scores {
                                let player_name = self.get_nickname(id);
                                let text = format!("{player_name}: {score}");
                                let text = RichText::new(text)
                                    .size(30.)
                                    .color(colors_for_player(id).captured);
                                ui.label(text);
                            }

                            let tick_ms = self.tick_duration.load(Ordering::Relaxed);
                            let mut slider_tick_ms = tick_ms;
                            ui.add(Slider::new(&mut slider_tick_ms, 0..=1000));
                            ui.label("Tick (ms)");
                            if slider_tick_ms != tick_ms {
                                self.tick_duration.store(slider_tick_ms, Ordering::Relaxed);
                            }
                        })
                    });

                    for (k, d) in KEY_MAP {
                        if ui.input(|i| i.key_pressed(k)) {
                            self.direction.store(d);
                        }
                    }
                }
                State::Ended => {
                    ui.label("Game ended");
                }
            }
            drop(state_guard);
        });
    }
}

struct AtomicDirection(Arc<AtomicU8>);

impl AtomicDirection {
    fn new(direction: Direction) -> Self {
        Self(Arc::new(AtomicU8::new(direction as u8)))
    }

    fn store(&self, direction: Direction) {
        self.0.store(direction as u8, Ordering::Relaxed);
    }

    fn load(&self) -> Direction {
        let direction = self.0.load(Ordering::Relaxed);
        Direction::from_u8(direction).unwrap()
    }

    fn clone(&self) -> AtomicDirection {
        Self(self.0.clone())
    }
}
