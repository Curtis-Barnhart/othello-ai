use std::fmt;

/// All 8 surrounding directions in a grid
static AROUND: [(u8, u8); 8] = [
    (255, 1),   (0, 1),   (1, 1),
    (255, 0),             (1, 0),
    (255, 255), (0, 255), (1, 255),
];

/// The two players in the game.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Players {
    White,
    Black,
}

/// The state of a board tile
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum States {
    /// Tile is taken by a [Player](Players).
    Taken(Players),
    /// Tile is empty.
    Empty,
}

/// Represents the game board: an 8x8 grid of tile states.
#[derive(Debug, Clone, Copy)]
pub struct Board {
    /// 8x8 grid of tile states.
    pub pieces: [[States; 8]; 8],
}


impl fmt::Display for Board {
    /// Formats the board as a human-readable string with coordinates and pieces.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            " 01234567\n{}",
            self.pieces.iter().enumerate().map(
                |(index, inner)| -> String {
                    index.to_string() + &inner.iter().map(
                        |tile: &States| -> String {
                            match tile {
                                States::Empty => String::from("."),
                                States::Taken(Players::Black) => String::from("B"),
                                States::Taken(Players::White) => String::from("W"),
                            }
                        }
                    ).collect::<Vec<String>>().join("")
                }
            ).collect::<Vec<String>>().join("\n")
        )
    }
}

/// Helper type used to describe flipping outcomes.
#[derive(Debug, PartialEq)]
enum FlipType {
    /// Flips nonzero amount of tiles.
    Valid,
    /// Flips zero tiles.
    Degenerate,
    /// Goes off the side of the board.
    Invalid,
}

impl Board {
    /// Creates a new board with all cells empty.
    pub fn new() -> Self {
        Board {
            pieces: [[States::Empty; 8]; 8],
        }
    }

    /// Returns the score of the board.
    ///
    /// Positive if Black is winning, negative if White is winning.
    pub fn score(&self) -> i8 {
        self.pieces.iter().map(
            |row: &[States; 8]| -> i8 {
                row.iter().map(
                    |piece: &States| -> i8 {
                        match piece {
                            States::Empty => 0,
                            States::Taken(Players::Black) => 1,
                            States::Taken(Players::White) => -1
                        }
                    }
                ).sum()
            }
        ).sum()
    }

    /// Sets the tile at `(x, y)` to a given [States] value.
    ///
    /// Does not perform bounds checking (may panic).
    pub fn change(&mut self, x: u8, y: u8, val: States) {
        self.pieces[usize::from(y)][usize::from(x)] = val;
    }

    /// Returns the tile at `(x, y)` or [None] if out of bounds.
    pub fn at(&self, x: u8, y: u8) -> Option<States> {
        if x < 8 && y < 8 {
            return Some(self.pieces[usize::from(y)][usize::from(x)]);
        }
        None
    }

    /// Returns a list of all valid moves for a given player.
    pub fn get_moves(&self, p: Players) -> Vec<(u8, u8)> {
        let mut v: Vec<(u8, u8)> = Vec::new();
        v.reserve(64);
        for x in 0..8 {
            for y in 0..8 {
                if self.can_move(x, y, p) {
                    v.push((x, y));
                }
            }
        }
        v.shrink_to_fit();
        v
    }

    /// Helper for determining if flipping from `(x, y)`
    /// towards the direction `(dx, dy)` is possible.
    ///
    /// Returns [Invalid](FlipType::Invalid) if the flip goes off the side of
    /// the board without ending, [Degenerate](FlipType::Degenerate) if
    /// the flip flips exactly 0 tiles, and [Valid](FlipType::Valid) otherwise.
    fn can_flip_toward_help(&self, x: u8, y: u8, dx: u8, dy: u8, origin: Players) -> FlipType {
        let new_x = x.wrapping_add(dx);
        let new_y = y.wrapping_add(dy);
        if let Some(States::Taken(new_player)) = self.at(new_x, new_y) {
            if origin != new_player {
                if self.can_flip_toward_help(new_x, new_y, dx, dy, origin) != FlipType::Invalid {
                    FlipType::Valid
                } else { FlipType::Invalid }
            } else { FlipType::Degenerate }
        } else { FlipType::Invalid }
    }

    /// Checks if flipping is valid in a given direction.
    ///
    /// Does not perform bounds checking - a tile that is not on the board
    /// but which flips onto the board will return [true].
    fn can_flip_toward(&self, x: u8, y: u8, dx: u8, dy: u8, origin: Players) -> bool {
        self.can_flip_toward_help(x, y, dx, dy, origin) == FlipType::Valid
    }

    /// Determines if a player can place a piece at `(x, y)`.
    pub fn can_move(&self, x: u8, y: u8, p: Players) -> bool {
        if let Some(States::Empty) = self.at(x, y) {
            for (dx, dy) in AROUND {
                if self.can_flip_toward(x, y, dx, dy, p) {
                    return true;
                }
            }
        }
        false
    }

    // does not check x and y values for being on board
    // If it goes off the side it returns None
    // If it has no opposite color in between it returns an empty vec
    /// Attempts to flip pieces from `(x, y)` (not inclusive)
    /// towards a certain direction `(dx, dy)`.
    /// Assumes that the [color](Players) of `(x, y)` is `origin`.
    ///
    /// Returns [None] if the flip would go off the board
    /// and a list of locations that would be flipped otherwise.
    /// Does not check bounds - a flip originating off the board
    /// but which flips onto the board will register as valid.
    fn flip_toward(&mut self, x: u8, y: u8, dx: u8, dy: u8, origin: Players) -> Option<Vec<(u8, u8)>> {
        let new_x = x.wrapping_add(dx);
        let new_y = y.wrapping_add(dy);
        if let Some(States::Taken(new_player)) = self.at(new_x, new_y) {
            if origin != new_player {
                if let Some(mut future_list) = self.flip_toward(new_x, new_y, dx, dy, origin) {
                    self.change(new_x, new_y, States::Taken(origin));
                    future_list.push((new_x, new_y));
                    Some(future_list)
                } else { None }
            } else { Some(Vec::new()) }
        } else {
            None
        }
    }

    /// Flips all valid pieces in every direction around `(x, y)`
    /// and returns a list of pieces that would be flipped.
    /// Assumes the move at `(x, y)` has already been made.
    ///
    /// If you do not want the list of flipped tiles but still want to check
    /// if the flip was valid, consider [Board::flip_all_fast].
    pub fn flip_all(&mut self, x: u8, y: u8) -> Vec<(u8, u8)> {
        let mut places: Vec<(u8, u8)> = Vec::new();
        if let Some(States::Taken(origin)) = self.at(x, y) {
            for (dx, dy) in AROUND {
                if let Some(partial) = self.flip_toward(x, y, dx, dy, origin) {
                    places.extend(partial);
                }
            }
        }
        places
    }

    /// Recursive helper for [Board::flip_toward_fast].
    fn flip_toward_fast_help(&mut self, x: u8, y: u8, dx: u8, dy: u8, origin: Players) -> FlipType {
        let new_x = x.wrapping_add(dx);
        let new_y = y.wrapping_add(dy);
        if let Some(States::Taken(new_player)) = self.at(new_x, new_y) {
            if origin != new_player {
                if self.flip_toward_fast_help(new_x, new_y, dx, dy, origin) != FlipType::Invalid {
                    self.change(new_x, new_y, States::Taken(origin));
                    FlipType::Valid
                } else { FlipType::Invalid }
            } else { FlipType::Degenerate }
        } else { FlipType::Invalid }
    }

    /// Returns whether a flip from `(x, y)` (not inclusive) towards the
    /// direction `(dx, dy)` is valid and flips tiles if it is valid.
    ///
    /// Returns [Invalid](FlipType::Invalid) if the flip goes off the side of
    /// the board without ending, [Degenerate](FlipType::Degenerate) if
    /// the flip flips exactly 0 tiles, and [Valid](FlipType::Valid) otherwise.
    ///
    /// Does not perform bounds checking - a tile that is not on the board
    /// but which flips onto the board will be considered valid.
    fn flip_toward_fast(&mut self, x: u8, y: u8, dx: u8, dy: u8, origin: Players) -> bool {
        self.flip_toward_fast_help(x, y, dx, dy, origin) == FlipType::Valid
    }

    /// Fast version of [Board::flip_all] that flips tiles and returns
    /// whether or not a flip was valid without returning the list of flipped
    /// tiles.
    ///
    /// Assumes the move at `(x, y)` has already been applied.
    pub fn flip_all_fast(&mut self, x: u8, y: u8) -> bool {
        if let Some(States::Taken(origin)) = self.at(x, y) {
            let mut any = false;
            for (dx, dy) in AROUND {
                if self.flip_toward_fast(x, y, dx, dy, origin) {
                    any = true;
                }
            }
            any
        } else {
            false
        }
    }
}
