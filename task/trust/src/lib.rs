#![forbid(unsafe_code)]

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundOutcome {
    BothCooperated,
    LeftCheated,
    RightCheated,
    BothCheated,
}

pub struct Game {
    left: Box<dyn Agent>,
    right: Box<dyn Agent>,
}

impl Game {
    pub fn new(left: Box<dyn Agent>, right: Box<dyn Agent>) -> Self {
        Self { left, right }
    }

    pub fn left_score(&self) -> i32 {
        self.left.get_score()
    }

    pub fn right_score(&self) -> i32 {
        self.right.get_score()
    }

    pub fn play_round(&mut self) -> RoundOutcome {
        const MUTUAL_COOP_DELTA: i32 = 2;
        const CHEAT_DELTA: i32 = 3;
        const COOPERATION_DELTA: i32 = -1;

        let left_move = self.left.play_round();
        let right_move = self.right.play_round();

        self.left.update(right_move);
        self.right.update(left_move);

        match (left_move, right_move) {
            (Move::Cooperate, Move::Cooperate) => {
                change_score(&mut self.left, MUTUAL_COOP_DELTA);
                change_score(&mut self.right, MUTUAL_COOP_DELTA);

                RoundOutcome::BothCooperated
            }
            (Move::Cooperate, Move::Cheat) => {
                change_score(&mut self.left, COOPERATION_DELTA);
                change_score(&mut self.right, CHEAT_DELTA);

                RoundOutcome::RightCheated
            }
            (Move::Cheat, Move::Cooperate) => {
                change_score(&mut self.left, CHEAT_DELTA);
                change_score(&mut self.right, COOPERATION_DELTA);

                RoundOutcome::LeftCheated
            }
            (Move::Cheat, Move::Cheat) => RoundOutcome::BothCheated,
        }
    }
}

fn change_score(agent: &mut Box<dyn Agent>, delta: i32) {
    let score = agent.get_score();
    agent.set_score(score + delta);
}

pub trait Agent {
    fn play_round(&mut self) -> Move;
    fn update(&mut self, opponent_move: Move);
    fn get_score(&self) -> i32;
    fn set_score(&mut self, score: i32);
}

#[derive(Debug, Clone, Copy)]
pub enum Move {
    Cooperate,
    Cheat,
}

////////////////////////////////////////////////////////////////////////////////

pub struct CheatingAgent {
    score: i32,
}

impl CheatingAgent {
    pub fn new() -> Self {
        Self { score: 0 }
    }
}

impl Default for CheatingAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for CheatingAgent {
    fn play_round(&mut self) -> Move {
        Move::Cheat
    }

    fn update(&mut self, _opponent_move: Move) {}

    fn get_score(&self) -> i32 {
        self.score
    }

    fn set_score(&mut self, score: i32) {
        self.score = score
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct CooperatingAgent {
    score: i32,
}

impl CooperatingAgent {
    pub fn new() -> Self {
        Self { score: 0 }
    }
}

impl Default for CooperatingAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for CooperatingAgent {
    fn play_round(&mut self) -> Move {
        Move::Cooperate
    }

    fn update(&mut self, _opponent_move: Move) {}

    fn get_score(&self) -> i32 {
        self.score
    }

    fn set_score(&mut self, score: i32) {
        self.score = score
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct GrudgerAgent {
    score: i32,
    holding_grudge: bool,
}

impl GrudgerAgent {
    pub fn new() -> Self {
        Self {
            score: 0,
            holding_grudge: false,
        }
    }
}

impl Default for GrudgerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for GrudgerAgent {
    fn play_round(&mut self) -> Move {
        match self.holding_grudge {
            true => Move::Cheat,
            false => Move::Cooperate,
        }
    }

    fn update(&mut self, opponent_move: Move) {
        if let Move::Cheat = opponent_move {
            self.holding_grudge = true
        }
    }

    fn get_score(&self) -> i32 {
        self.score
    }

    fn set_score(&mut self, score: i32) {
        self.score = score
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct CopycatAgent {
    score: i32,
    latest_opponent_move: Option<Move>,
}

impl CopycatAgent {
    pub fn new() -> Self {
        Self {
            score: 0,
            latest_opponent_move: None,
        }
    }
}

impl Default for CopycatAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for CopycatAgent {
    fn play_round(&mut self) -> Move {
        match self.latest_opponent_move {
            Some(Move::Cooperate) => Move::Cooperate,
            Some(Move::Cheat) => Move::Cheat,
            None => Move::Cooperate,
        }
    }

    fn update(&mut self, opponent_move: Move) {
        self.latest_opponent_move = Some(opponent_move)
    }

    fn get_score(&self) -> i32 {
        self.score
    }

    fn set_score(&mut self, score: i32) {
        self.score = score
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct DetectiveAgent {
    score: i32,
    turn_number: usize,
    initial_stage_combo: [Move; 4],
    opponent_cheated_during_initial_stage: bool,
    latest_opponent_move: Option<Move>,
}

impl DetectiveAgent {
    pub fn new() -> Self {
        Self {
            score: 0,
            turn_number: 0,
            initial_stage_combo: [
                Move::Cooperate,
                Move::Cheat,
                Move::Cooperate,
                Move::Cooperate,
            ],
            opponent_cheated_during_initial_stage: false,
            latest_opponent_move: None,
        }
    }
}

impl Default for DetectiveAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for DetectiveAgent {
    fn play_round(&mut self) -> Move {
        let result: Move = if self.turn_number < self.initial_stage_combo.len() {
            self.initial_stage_combo[self.turn_number]
        } else if self.opponent_cheated_during_initial_stage {
            self.latest_opponent_move
                .expect("latest_opponent_move must be Some at this point")
        } else {
            Move::Cheat
        };

        self.turn_number += 1;

        result
    }

    fn update(&mut self, opponent_move: Move) {
        if self.turn_number < self.initial_stage_combo.len() {
            if let Move::Cheat = opponent_move {
                self.opponent_cheated_during_initial_stage = true;
            }
        } else if self.opponent_cheated_during_initial_stage {
            self.latest_opponent_move = Some(opponent_move)
        }
    }

    fn get_score(&self) -> i32 {
        self.score
    }

    fn set_score(&mut self, score: i32) {
        self.score = score
    }
}
