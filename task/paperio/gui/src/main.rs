use num_traits::FromPrimitive;
use std::{
    io::{BufReader, BufWriter, Write},
    net::TcpStream,
    sync::{
        atomic::{AtomicU8, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

use clap::Parser;
use eframe::egui;
use egui::{pos2, vec2, Color32, Rect, Vec2};
use paperio_proto::{
    traits::{ProtoRead, ProtoWrite},
    Cell, CommandMessage, Direction, GameParams, Message, PlayerId, World,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Arguments {
    #[arg(short, long, default_value_t = String::from("localhost"))]
    address: String,
    #[arg(short, long, default_value_t = 8000)]
    port: u16,
    #[arg(short, long, default_value_t = 120)]
    tick_delay_ms: u64,
}

fn main() {
    let args = Arguments::parse();

    let stream = TcpStream::connect(format!("{}:{}", args.address, args.port))
        .expect("failed to connect to tcp socket");
    let (msg_to_gui, msg_receiver) = mpsc::channel();
    let atomic_direction_store = AtomicDirection::new(Direction::Left);

    // thread for sending and receiving messages
    let handle = spawn_cmd_thread(
        stream,
        msg_to_gui,
        atomic_direction_store.clone(),
        args.tick_delay_ms,
    );

    // run gui in current thread
    let native_options = eframe::NativeOptions {
        window_builder: Some(Box::new(|b| b.with_inner_size((1200., 960.)))),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "paperio",
        native_options,
        Box::new(|cc| {
            Ok(Box::new(PaperioApp::new(
                cc,
                msg_receiver,
                atomic_direction_store,
            )))
        }),
    );

    let _ = handle.join();
}

fn spawn_cmd_thread(
    stream: TcpStream,
    msg_to_gui: mpsc::Sender<Message>,
    atomic_direction_store: AtomicDirection,
    tick_delay_ms: u64,
) -> thread::JoinHandle<anyhow::Result<()>> {
    thread::spawn(move || -> anyhow::Result<()> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut writer = BufWriter::new(stream);

        // receive `GameParams` msg
        if let Ok(msg) = Message::read(&mut reader) {
            msg_to_gui.send(msg)?;
        } else {
            panic!("Could not read first message")
        };

        // receive tick msgs
        while let Ok(msg) = Message::read(&mut reader) {
            msg_to_gui.send(msg)?;

            thread::sleep(Duration::from_millis(tick_delay_ms));

            let direction = atomic_direction_store.load();
            let msg = CommandMessage {
                tick_num: 0,
                command: direction,
            };
            msg.write(&mut writer)?;
            writer.flush()?;
        }
        Ok(())
    })
}

#[derive(Clone, Copy)]
struct PlayerColors {
    head: Color32,
    captured: Color32,
    traced: Color32,
}

const COLOR_PALETTE: [PlayerColors; 5] = [
    PlayerColors {
        head: Color32::DARK_GREEN,
        captured: Color32::GREEN,
        traced: Color32::LIGHT_GREEN,
    },
    PlayerColors {
        head: Color32::from_rgb(191, 2, 71),
        captured: Color32::from_rgb(216, 27, 96),
        traced: Color32::from_rgb(231, 114, 156),
    },
    PlayerColors {
        head: Color32::from_rgb(220, 99, 0),
        captured: Color32::from_rgb(245, 124, 0),
        traced: Color32::from_rgb(249, 174, 97),
    },
    PlayerColors {
        head: Color32::from_rgb(71, 100, 114),
        captured: Color32::from_rgb(96, 125, 139),
        traced: Color32::from_rgb(156, 174, 183),
    },
    PlayerColors {
        head: Color32::from_rgb(65, 134, 128),
        captured: Color32::from_rgb(90, 159, 153),
        traced: Color32::from_rgb(154, 195, 192),
    },
];

fn colors_for_player(id: &PlayerId) -> PlayerColors {
    match id as &str {
        "1" => COLOR_PALETTE[1],
        "2" => COLOR_PALETTE[2],
        "3" => COLOR_PALETTE[3],
        "4" => COLOR_PALETTE[4],
        _ => COLOR_PALETTE[0],
    }
}

fn head_color(id: &PlayerId) -> Color32 {
    colors_for_player(id).head
}

fn cell_color(s: &CellState) -> Color32 {
    match s {
        CellState::Free => Color32::WHITE,
        CellState::Captured(id) => colors_for_player(id).captured,
        CellState::Trace(id) => colors_for_player(id).traced,
    }
}

#[derive(Debug, Clone)]
enum CellState {
    Free,
    Captured(PlayerId),
    Trace(PlayerId),
}

struct GameField {
    params: GameParams,
    cells: Vec<Vec<CellState>>,
    world: World,
}

impl GameField {
    fn new(params: GameParams) -> Self {
        let cells = vec![
            vec![CellState::Free; params.x_cells_count as usize];
            params.y_cells_count as usize
        ];
        Self {
            params,
            cells,
            world: World {
                players: Default::default(),
                tick_num: 0,
            },
        }
    }

    fn clear_field(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                *cell = CellState::Free;
            }
        }
    }

    fn update(&mut self, world: World) {
        self.clear_field();
        for (id, p) in world.players.iter() {
            for &Cell(x, y) in p.territory.iter() {
                self.cells[y as usize][x as usize] = CellState::Captured(id.clone());
            }
        }
        for (id, p) in world.players.iter() {
            for &Cell(x, y) in p.lines.iter() {
                self.cells[y as usize][x as usize] = CellState::Trace(id.clone());
            }
        }
        self.world = world;
    }

    fn draw(&self, ui: &mut egui::Ui) {
        let cell_size = ui.available_size()
            / vec2(
                self.params.x_cells_count as f32,
                self.params.y_cells_count as f32,
            );
        let cell_size = cell_size.min_elem();
        let cell_sizes = Vec2::splat(cell_size - 1.);

        let painter = ui.painter();
        let draw_cell = |c: Cell, color: Color32| {
            let Cell(x, y) = c;
            let y = self.params.y_cells_count - 1 - y as u32;
            let pos = pos2(x as f32, y as f32) * cell_size;
            let r = Rect::from_min_size(pos, cell_sizes);
            painter.rect_filled(r, 0., color);
        };

        for (y, row) in self.cells.iter().enumerate() {
            for (x, c) in row.iter().enumerate() {
                let color = cell_color(c);
                draw_cell(Cell(x as i32, y as i32), color)
            }
        }
        for (id, player) in &self.world.players {
            if !player.has_lost {
                let color = head_color(id);
                draw_cell(player.position, color)
            }
        }
    }
}

enum State {
    AwaitForGameStart,
    AwaitForFirstTick(GameParams),
    Tick(GameField),
    Ended,
}

struct AtomicDirection(Arc<AtomicU8>);

impl AtomicDirection {
    fn new(direction: Direction) -> Self {
        Self(Arc::new(AtomicU8::new(direction as u8)))
    }

    fn store(&mut self, direction: Direction) {
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

struct PaperioApp {
    state: State,
    msg_receiver: mpsc::Receiver<Message>,
    direction: AtomicDirection,
}

impl PaperioApp {
    fn new(
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
}

impl PaperioApp {
    fn update_state(&mut self, read_message: Message) {
        match read_message {
            Message::StartGame(game_params) => self.state = State::AwaitForFirstTick(game_params),
            Message::Tick(world) => {
                match &mut self.state {
                    State::AwaitForGameStart => todo!("tick while Await"),
                    State::AwaitForFirstTick(params) => {
                        let mut field = GameField::new(*params);
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
}

const KEY_MAP: [(egui::Key, Direction); 4] = [
    (egui::Key::ArrowUp, Direction::Up),
    (egui::Key::ArrowDown, Direction::Down),
    (egui::Key::ArrowRight, Direction::Right),
    (egui::Key::ArrowLeft, Direction::Left),
];

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
                    game.draw(ui);
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
