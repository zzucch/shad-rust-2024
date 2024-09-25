use crate::{
    colors::{cell_color, colors_for_player, head_color},
    state::GameState,
};
use std::sync::{
    atomic::{AtomicU8, Ordering},
    mpsc, Arc,
};

use eframe::egui;
use egui::{pos2, vec2, Align, Color32, Layout, Rect, RichText, Sense, Vec2};
use num_traits::FromPrimitive;
use paperio_proto::{Cell, Direction, GameParams, Message};

const KEY_MAP: [(egui::Key, Direction); 4] = [
    (egui::Key::ArrowUp, Direction::Up),
    (egui::Key::ArrowDown, Direction::Down),
    (egui::Key::ArrowRight, Direction::Right),
    (egui::Key::ArrowLeft, Direction::Left),
];

const CELL_SIZE_WITH_BORDER: f32 = 31.;
const CELL_SIZES: Vec2 = Vec2::splat(CELL_SIZE_WITH_BORDER - 1.);

pub struct AtomicDirection(Arc<AtomicU8>);

impl AtomicDirection {
    pub fn new(direction: Direction) -> Self {
        Self(Arc::new(AtomicU8::new(direction as u8)))
    }

    pub fn store(&mut self, direction: Direction) {
        self.0.store(direction as u8, Ordering::Relaxed);
    }

    pub fn load(&self) -> Direction {
        let direction = self.0.load(Ordering::Relaxed);
        Direction::from_u8(direction).unwrap()
    }

    pub fn clone(&self) -> AtomicDirection {
        Self(self.0.clone())
    }
}

enum State {
    AwaitForGameStart,
    AwaitForFirstTick(GameParams),
    Tick(GameState),
    Ended,
}

pub struct PaperioApp {
    state: State,
    msg_receiver: mpsc::Receiver<Message>,
    direction: AtomicDirection,
}

impl PaperioApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        reader: mpsc::Receiver<Message>,
        dir: AtomicDirection,
    ) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self {
            state: State::AwaitForGameStart,
            msg_receiver: reader,
            direction: dir,
        }
    }

    fn update_state(&mut self, read_message: Message) {
        match read_message {
            Message::StartGame(game_params) => self.state = State::AwaitForFirstTick(game_params),
            Message::Tick(world) => {
                match &mut self.state {
                    State::AwaitForGameStart => todo!("tick while Await"),
                    State::AwaitForFirstTick(params) => {
                        let mut field = GameState::new(*params);
                        field.update(world);
                        self.state = State::Tick(field);
                    }
                    State::Tick(game_field) => {
                        game_field.update(world);
                    }
                    State::Ended => todo!("tick while Ended"),
                };
            }
            Message::EndGame {} => self.state = State::Ended,
        };
    }

    fn draw_field(&self, ui: &mut egui::Ui, game: &GameState) {
        let params = game.params;
        let size_in_cells = vec2(params.x_cells_count as f32, params.y_cells_count as f32);

        let (_, painter) =
            ui.allocate_painter(size_in_cells * CELL_SIZE_WITH_BORDER, Sense::hover());

        let zero_pos = ui.min_rect().min.to_vec2();
        let draw_cell = |Cell(x, y): Cell, color: Color32| {
            // Game indexation is down-to-top, but we draw top-to-down, so invert here.
            let y = params.y_cells_count - 1 - y as u32;
            let rect_corner = pos2(x as f32, y as f32) * CELL_SIZE_WITH_BORDER + zero_pos;
            let rect = Rect::from_min_size(rect_corner, CELL_SIZES);
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
            if let Ok(msg) = self.msg_receiver.try_recv() {
                self.update_state(msg)
            }
            match self.state {
                State::AwaitForGameStart => {
                    ui.label("Waiting to 'start_game'");
                }
                State::AwaitForFirstTick(_) => {
                    ui.label("Game started, waiting for the first tick");
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
                                let text = if id.as_str() == "i" {
                                    format!("Me: {score}")
                                } else {
                                    format!("Player #{id}: {score}")
                                };
                                let text = RichText::new(text)
                                    .size(30.)
                                    .color(colors_for_player(id).captured);
                                ui.label(text);
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
        });
    }
}
