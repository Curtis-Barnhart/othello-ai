use std::fmt;

pub use crate::mechanics::{Players, States, Board};

pub type Turn = Option<(u8, u8)>;

#[derive(Clone)]
pub struct Gamestate {
    board: Board,
    turn: u8,
    moves: Option<Vec<Turn>>,
}

impl fmt::Display for Gamestate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}\n{}",
            self.board,
            match self.whose_turn() {
                States::Empty => "Game Over",
                States::Taken(Players::Black) => "Black to play",
                States::Taken(Players::White) => "White to play",
            },
        )
    }
}

impl Gamestate {
    pub fn new() -> Self {
        let mut g = Gamestate {
            board: Board::new(),
            turn: 0,
            moves: None,
        };
        g.board.pieces[3][3] = States::Taken(Players::White);
        g.board.pieces[4][4] = States::Taken(Players::White);
        g.board.pieces[4][3] = States::Taken(Players::Black);
        g.board.pieces[3][4] = States::Taken(Players::Black);
        g
    }

    fn raw_turn(&self) -> Players {
        if self.turn & 1 == 0 {
            Players::Black
        } else {
            Players::White
        }
    }

    pub fn whose_turn(&self) -> States {
        if self.is_terminal() {
            States::Empty
        } else {
            if self.turn & 1 == 0 {
                States::Taken(Players::Black)
            } else {
                States::Taken(Players::White)
            }
        }
    }

    pub fn score(&self) -> i8 {
        self.board.score()
    }

    // If the game is over, returns None
    // If pass is the only move, empty list
    // otherwise, list of moves
    pub fn get_moves(&self) -> Vec<Turn> {
        if let States::Taken(whose) = self.whose_turn() {
            let moves = self.board.get_moves(whose);
            if moves.is_empty() {
                vec![None]
            } else {
                moves.into_iter().map(
                    |t| { Some(t) }
                ).collect()
            }
        } else {
            Vec::new()
        }
    }

    pub fn valid_move(&self, m: Turn) -> bool {
        if let States::Taken(_) = self.whose_turn() {
            self.get_moves().contains(&m)
        } else {
            false
        }
    }

    pub fn view_board(&self) -> &crate::mechanics::Board {
        &self.board
    }

    pub fn is_terminal(&self) -> bool {
        let whose_turn = self.raw_turn();
        let moves = self.board.get_moves(whose_turn);
        match (moves.is_empty(), whose_turn) {
            (false, _) => false,
            (true, Players::Black) => self.board.get_moves(Players::White).is_empty(),
            (true, Players::White) => self.board.get_moves(Players::Black).is_empty(),
        }
    }

    // Returns None if the game is over
    pub fn make_move(&mut self, turn: Turn) -> Option<Vec<(u8, u8)>> {
        if let States::Taken(whose_turn) = self.whose_turn() {
            if self.get_moves().contains(&turn) {
                self.turn += 1;
                if let Some((x, y)) = turn {
                    self.board.change(x, y, States::Taken(whose_turn));
                    Some(self.board.flip_all(x, y))
                } else {
                    Some(Vec::new())
                }
            } else { None }
        } else { None }
    }

    // Returns None if the game is over
    pub fn make_move_fast(&mut self, turn: Turn) -> bool {
        if let States::Taken(whose_turn) = self.whose_turn() {
            if self.get_moves().contains(&turn) {
                self.turn += 1;
                if let Some((x, y)) = turn {
                    self.board.change(x, y, States::Taken(whose_turn));
                    self.board.flip_all_fast(x, y);
                }
                true
            } else { false }
        } else { false }
    }

    // does not check if the game is over
    pub fn make_moves_fast(&mut self, turns: &[Turn]) -> bool {
        for t in turns {
            if !self.make_move_fast(*t) { return false; }
        }
        true
    }
}

pub fn str_to_loc(s: &str) -> Turn {
    let stripped = s.replace(" ", "");
    let mut iter = stripped.split(",");
    if let (Some(x), Some(y)) = (iter.next(), iter.next()) {
        if let (Ok(x), Ok(y)) = (x.parse::<u8>(), y.parse::<u8>()) {
            Some((x, y))
        } else { None }
    } else { None }
}
