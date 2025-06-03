use std::fmt;

pub use crate::mechanics::Players;
pub use crate::mechanics::States;

#[derive(Clone)]
pub struct Gamestate {
    board: crate::mechanics::Board,
    history: Vec<(u8, u8)>,
}

impl fmt::Display for Gamestate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}\n{} to move",
            self.board,
            if self.whose_turn() == Players::Black { "Black" } else { "White" },
        )
    }
}

impl Gamestate {
    pub fn new() -> crate::gameplay::Gamestate {
        let mut g = Gamestate {
            board: crate::mechanics::Board::new(),
            history: Vec::new(),
        };
        g.board.change(3, 3, States::Taken(Players::White));
        g.board.change(4, 4, States::Taken(Players::White));
        g.board.change(4, 3, States::Taken(Players::Black));
        g.board.change(3, 4, States::Taken(Players::Black));
        g
    }

    pub fn whose_turn(&self) -> Players {
        if self.history.len() & 1 == 0 { Players::Black } else { Players::White }
    }

    pub fn score(&self) -> i8 {
        self.board.score()
    }

    pub fn get_moves(&self) -> Vec<(u8, u8)> {
        self.board.get_moves(self.whose_turn())
    }

    pub fn view_board(&self) -> &crate::mechanics::Board {
        &self.board
    }

    pub fn view_history(&self) -> &Vec<(u8, u8)> {
        &self.history
    }

    pub fn make_turn(&mut self, x: u8, y: u8) -> Vec<(u8, u8)> {
        if let Some(States::Empty) = self.board.at(x, y) {
            self.board.change(x, y, States::Taken(self.whose_turn()));
            let v = self.board.flip_all(x, y);
            if v.is_empty() {
                self.board.change(x, y, States::Empty);
            } else {
                self.history.push((x, y));
            }
            v
        } else {
            Vec::new()
        }
    }

    pub fn make_turns(&mut self, turns: &[(u8, u8)]) -> bool {
        for (x, y) in turns {
            let flips = self.make_turn(*x, *y);
            if flips.is_empty() { return false; }
        }
        true
    }
}

pub fn str_to_loc(s: &str) -> Option<(u8, u8)> {
    let stripped = s.replace(" ", "");
    let mut iter = stripped.split(",");
    if let (Some(x), Some(y)) = (iter.next(), iter.next()) {
        if let (Ok(x), Ok(y)) = (x.parse::<u8>(), y.parse::<u8>()) {
            Some((x, y))
        } else { None }
    } else { None }
}
