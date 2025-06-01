mod mechanics {
    use std::fmt;

    static AROUND: [(u8, u8); 8] = [
        (255, 1),   (0, 1),   (1, 1),
        (255, 0),             (1, 0),
        (255, 255), (0, 255), (1, 255),
    ];

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Players {
        White,
        Black,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum States {
        Taken(Players),
        Empty,
    }

    #[derive(Debug)]
    pub struct Board {
        pieces: [[States; 8]; 8],
    }

    impl fmt::Display for Board {
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

    #[derive(Debug, PartialEq)]
    enum FlipType {
        Valid,
        Degenerate,
        Invalid,
    }

    impl Board {
        pub fn new() -> Self {
            Board {
                pieces: [[States::Empty; 8]; 8],
            }
        }

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

        // does not check x and y
        pub fn change(&mut self, x: u8, y: u8, val: States) {
            self.pieces[usize::from(y)][usize::from(x)] = val;
        }

        // Returns None if location is off the board
        pub fn at(&self, x: u8, y: u8) -> Option<States> {
            if x < 8 && y < 8 {
                return Some(self.pieces[usize::from(y)][usize::from(x)]);
            }
            None
        }

        // does not check x and y values for being on board
        // If it goes off the side it returns None
        // If it has no opposite color in between it returns Some(false)
        fn can_flip_toward_help(&self, x: u8, y: u8, dx: u8, dy: u8, origin: Players) -> FlipType {
            let new_x = x.wrapping_add(dx);
            let new_y = y.wrapping_add(dy);
            if let Some(States::Taken(new_player)) = self.at(new_x, new_y) {
                if origin != new_player {
                    if let FlipType::Degenerate = self.can_flip_toward_help(new_x, new_y, dx, dy, origin) {
                        FlipType::Valid
                    } else { FlipType::Invalid }
                } else { FlipType::Degenerate }
            } else { FlipType::Invalid }
        }

        // does not check x and y values for being on board
        fn can_flip_toward(&self, x: u8, y: u8, dx: u8, dy: u8, origin: Players) -> bool {
            self.can_flip_toward_help(x, y, dx, dy, origin) == FlipType::Valid
        }

        // handles all values of x and y
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

        // does not check x and y values for being on board
        // If it goes off the side it returns None
        // If it has no opposite color in between it returns an empty vec
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

        // handles any values of x and y
        // assumes the origin move has been made
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
    }
}

pub mod gameplay {
    use std::fmt;

    pub use crate::mechanics::Players;
    pub use crate::mechanics::States;

    pub struct Gamestate {
        board: crate::mechanics::Board,
        turn: u8,
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
                turn: 0,
            };
            g.board.change(3, 3, States::Taken(Players::White));
            g.board.change(4, 4, States::Taken(Players::White));
            g.board.change(4, 3, States::Taken(Players::Black));
            g.board.change(3, 4, States::Taken(Players::Black));
            g
        }

        pub fn whose_turn(&self) -> Players {
            if self.turn & 1 == 0 { Players::Black } else { Players::White }
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

        pub fn make_turn(&mut self, x: u8, y: u8) -> Vec<(u8, u8)> {
            if let Some(States::Empty) = self.board.at(x, y) {
                self.board.change(x, y, States::Taken(self.whose_turn()));
                let v = self.board.flip_all(x, y);
                if v.is_empty() {
                    self.board.change(x, y, States::Empty);
                } else {
                    self.turn += 1;
                }
                v
            } else {
                Vec::new()
            }
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
}

pub mod agent {
    use crate::gameplay;

    pub trait Agent {
        fn make_move(&self, state: gameplay::Gamestate) -> (u8, u8);
    }

    pub struct RandomAgent {}

    impl Agent::make_move for RandomAgent {
        fn make_move(&self, state: gameplay::Gamestate) -> (u8, u8) {

        }
    }
}

use std::io;
fn main() {
    let stdin = io::stdin();

    let mut g = crate::gameplay::Gamestate::new();
    let mut input = String::new();
    loop {
        let valid_moves = g.get_moves();
        if valid_moves.is_empty() {
            println!("Game over - score: {}", g.score());
            break;
        }

        println!("{}", g);
        loop {
            println!("Enter a coordinate:");
            input.clear();
            stdin.read_line(&mut input).expect("stdio could not be read from");
            input.pop();

            if let Some((x, y)) = crate::gameplay::str_to_loc(&input) {
                let flipped = g.make_turn(x, y);
                if flipped.is_empty() {
                    println!("Not a valid move!");
                    continue;
                } else {
                    break;
                }
            } else {
                println!("Could not parse coordinate!");
            }
        };
    }
}
