use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

pub use crate::mechanics::{Players, States};
use crate::mechanics::Board;

/// A player's move, which may be a board position `(x, y)` or [None] for pass.
pub type Turn = Option<(u8, u8)>;

/// A representation of the game state, including the board, turn number,
/// and cached list of valid moves for the current player.
// TODO: hey make it so that when it clones it keeps the turn list (if it doesn't already?)
#[derive(Clone, Debug, PartialEq)]
pub struct Gamestate {
    board: Board,
    turn: u8,
    moves: RefCell<Option<Rc<Vec<Turn>>>>,
}

impl fmt::Display for Gamestate {
    /// Formats the board followed by a message indicating whose turn it is,
    /// or "Game Over" if the game has ended.
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
    /// Constructs a new game state with the standard initial board configuration.
    ///
    /// If you desire to create a new game state with a custom initial
    /// configuration, consider [Gamestate::new_mock].
    pub fn new() -> Self {
        let mut g = Gamestate {
            board: Board::new(),
            turn: 0,
            moves: RefCell::new(None),
        };
        g.board.pieces[3][3] = States::Taken(Players::White);
        g.board.pieces[4][4] = States::Taken(Players::White);
        g.board.pieces[4][3] = States::Taken(Players::Black);
        g.board.pieces[3][4] = States::Taken(Players::Black);
        g
    }

    /// Constructs a game state with a given board and turn value.
    /// Useful for testing or simulation purposes.
    pub fn new_from(board: Board, turn: u8) -> Self {
        Gamestate {
            board: board,
            turn: turn,
            moves: RefCell::new(None),
        }
    }

    /// Returns whose turn it is.
    /// Returns [empty](States::Empty) if the game is over.
    pub fn whose_turn(&self) -> States {
        if self.get_moves().is_empty() {
            States::Empty
        } else {
            if self.turn & 1 == 0 {
                States::Taken(Players::Black)
            } else {
                States::Taken(Players::White)
            }
        }
    }

    /// Returns the score of the current board.
    /// Positive means Black is winning, negative means White is winning.
    pub fn score(&self) -> i8 {
        self.board.score()
    }

    /// Returns a reference-counted list of all valid moves
    /// (including [None] for pass).
    /// Cached after first computation for performance.
    pub fn get_moves(&self) -> Rc<Vec<Turn>> {
        if self.moves.borrow().is_none() {
            *self.moves.borrow_mut() = Some(Rc::new(self.gen_moves()));
        };
        self.moves.borrow().as_ref().unwrap().clone()
    }

    /// Generates the list of valid moves for the current player.
    /// If no moves are possible, returns a list containing only [None] (pass).
    /// If the game is over, returns an empty list.
    fn gen_moves(&self) -> Vec<Turn> {
        let possible_turn = if self.turn & 1 == 0 {
            Players::Black
        } else {
            Players::White
        };

        let moves = self.board.get_moves(possible_turn);
        let is_terminal = match (moves.is_empty(), possible_turn) {
            (false, _) => false,
            (true, Players::Black) => self.board.get_moves(Players::White).is_empty(),
            (true, Players::White) => self.board.get_moves(Players::Black).is_empty(),
        };

        if is_terminal {
            Vec::new()
        } else {
            if moves.is_empty() {
                vec![None]
            } else {
                moves.into_iter().map(
                    |t| { Some(t) }
                ).collect()
            }
        }
    }

    /// Returns `true` if the move is valid for the current player.
    pub fn valid_move(&self, m: Turn) -> bool {
        self.get_moves().contains(&m)
    }

    /// Provides a shared reference to the underlying board.
    pub fn board(&self) -> &crate::mechanics::Board {
        &self.board
    }

    /// Applies the given move to the game state using full flipping logic.
    /// Returns a vector of flipped positions if successful,
    /// or [None] if invalid or game is over.
    ///
    /// If you do not want to see the list of flipped positions,
    /// consider [Gamestate::make_move_fast].
    pub fn make_move(&mut self, turn: Turn) -> Option<Vec<(u8, u8)>> {
        if let States::Taken(whose_turn) = self.whose_turn() {
            if self.get_moves().contains(&turn) {
                self.turn += 1;
                *self.moves.borrow_mut() = None;
                if let Some((x, y)) = turn {
                    self.board.change(x, y, States::Taken(whose_turn));
                    Some(self.board.flip_all(x, y))
                } else {
                    Some(Vec::new())
                }
            } else { None }
        } else { None }
    }

    /// Applies the given move to the game state using full flipping logic.
    /// Unlike [Gamestate::make_move], does not return the list of flipped
    /// tiles.
    /// If the move does not go through, maintains original board state.
    ///
    /// Returns [true} if the move was valid and applied, [false] otherwise.
    pub fn make_move_fast(&mut self, turn: Turn) -> bool {
        if let States::Taken(whose_turn) = self.whose_turn() {
            if self.get_moves().contains(&turn) {
                self.turn += 1;
                if let Some((x, y)) = turn {
                    self.board.change(x, y, States::Taken(whose_turn));
                    self.board.flip_all_fast(x, y);
                }
                *self.moves.borrow_mut() = None;
                true
            } else { false }
        } else { false }
    }

    /// Applies a sequence of moves and reports whether all moves were valid.
    /// Returns [false] on the first invalid move.
    ///
    /// Note that this method does not rollback valid moves if an invalid move
    /// is encountered.
    pub fn make_moves_fast(&mut self, turns: &[Turn]) -> bool {
        for t in turns {
            if !self.make_move_fast(*t) { return false; }
        }
        true
    }
}

/// Converts a string matching " *\d *, *\d *" into a tuple of ints.
/// Does check that they are less than 8.
///
/// Returns [None] if parsing fails or the format is incorrect.
pub fn str_to_loc(s: &str) -> Option<(u8, u8)> {
    let stripped = s.replace(" ", "");
    let mut iter = stripped.split(",");
    if let (Some(x), Some(y)) = (iter.next(), iter.next()) {
        if let (Ok(x), Ok(y)) = (x.parse::<u8>(), y.parse::<u8>()) {
            if x < 8 && y < 8 {
                Some((x, y))
            } else { None }
        } else { None }
    } else { None }
}
