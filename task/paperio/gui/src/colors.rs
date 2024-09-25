use egui::Color32;
use paperio_proto::PlayerId;

use crate::state::CellState;

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

#[derive(Clone, Copy)]
pub struct PlayerColors {
    pub head: Color32,
    pub captured: Color32,
    pub traced: Color32,
}

pub fn colors_for_player(id: &PlayerId) -> PlayerColors {
    match id as &str {
        "1" => COLOR_PALETTE[1],
        "2" => COLOR_PALETTE[2],
        "3" => COLOR_PALETTE[3],
        "4" => COLOR_PALETTE[4],
        _ => COLOR_PALETTE[0],
    }
}

pub fn head_color(id: &PlayerId) -> Color32 {
    colors_for_player(id).head
}

pub fn cell_color(s: &CellState) -> Color32 {
    match s {
        CellState::Free => Color32::WHITE,
        CellState::Captured(id) => colors_for_player(id).captured,
        CellState::Trace(id) => colors_for_player(id).traced,
    }
}
