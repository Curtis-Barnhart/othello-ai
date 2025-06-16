use rand::prelude::IndexedRandom;
use rand::rngs::ThreadRng;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::io;

use crate::agent::{Agent, MemoryAgent};
use crate::gameplay::{Gamestate, Turn};

pub struct RandomAgent {
    r: RefCell<ThreadRng>,
}

impl RandomAgent {
    pub fn new() -> Self {
        RandomAgent {r: RefCell::new(rand::rng())}
    }
}

impl Agent for RandomAgent {
    fn make_move(&self, state: &Gamestate) -> Turn {
        let valid_moves = state.get_moves();
        valid_moves.choose(&mut self.r.borrow_mut())
                   .copied()
                   .expect("make_move passed state with no moves.")
    }
}

pub struct GreedyAgent {}

impl Agent for GreedyAgent {
    fn make_move(&self, state: &Gamestate) -> Turn {
        state.get_moves()
             .iter()
             .max_by(|t1, t2| -> Ordering {
                 // TODO: figure out wth derefing does to borrowing
                 let v1 = state.clone().make_move(**t1).expect("").len();
                 let v2 = state.clone().make_move(**t2).expect("").len();
                 v1.cmp(&v2)
             })
            .copied()
            .expect("make_moves passed state with no moves.")
    }
}

pub struct MemoryHumanAgent {
    game: Gamestate,
}

impl MemoryHumanAgent {
    pub fn new() -> Self {
        MemoryHumanAgent { game: Gamestate::new() }
    }
}

impl Agent for MemoryHumanAgent {
    fn make_move(&self, state: &Gamestate) -> Turn {
        let stdin = io::stdin();
        let mut input = String::new();
        let valid_moves = state.get_moves();
        println!("{}", state);

        if valid_moves.is_empty() {
            panic!("make_move passed state with no moves.");
        }

        if valid_moves.contains(&None) {
            println!("No available moves - return to pass:");
            stdin.read_line(&mut input).expect("stdio could not be read from");
            None
        } else {
            loop {
                println!("Enter a coordinate:");
                input.clear();
                stdin.read_line(&mut input).expect("stdio could not be read from");
                input.pop();

                if let Some(location) = crate::gameplay::str_to_loc(&input) {
                    if valid_moves.contains(&Some(location)) {
                        break Some(location)
                    } else {
                        println!("Not a valid move!");
                    }
                } else {
                    println!("Could not parse coordinate!");
                }
            }
        }
    }
}

impl MemoryAgent for MemoryHumanAgent {
    fn initialize_game(&mut self, state: Gamestate) {
        self.game = state;
    }

    fn make_move(&mut self) -> Turn {
        let turn = Agent::make_move(self, &self.game);
        self.game.make_move(turn);
        turn
    }

    fn opponent_move(&mut self, op: &Turn) {
        self.game.make_move(*op);
    }
}

pub struct HumanDebugger {}

impl Agent for HumanDebugger {
    fn make_move(&self, state: &Gamestate) -> Turn {
        let stdin = io::stdin();
        let mut input = String::new();
        let valid_moves = state.get_moves();
        println!("{}", state);

        if valid_moves.contains(&None) {
            loop {
                println!("Only valid move is to pass. Return to confirm:");
                input.clear();
                stdin.read_line(&mut input).expect("stdio could not be read from");
                input.pop();

                if input == "/moves" {
                    println!("There are no valid moves besides passing your turn");
                } else if input == "/history" {
                    println!("This is a reminder to fix the history feature");
                    //                println!("{}", state.view_history().iter().map(
                    //                        |(x, y)| -> String { format!("({}, {})", x, y) }
                    //                ).collect::<Vec<String>>().join(", "));
                } else {
                    break None;
                }
            }
        } else {
            loop {
                println!("Enter a coordinate:");
                input.clear();
                stdin.read_line(&mut input).expect("stdio could not be read from");
                input.pop();

                if input == "/moves" {
                    println!("{}", valid_moves.iter().map(
                            |turn| -> String {
                                if let Some((x, y)) = turn {
                                    format!("({}, {})", x, y) 
                                } else {
                                    format!("(Pass)")
                                }
                            }
                    ).collect::<Vec<String>>().join(", "));
                } else if input == "/history" {
                    println!("This is a reminder to fix the history feature");
                    //                println!("{}", state.view_history().iter().map(
                    //                        |(x, y)| -> String { format!("({}, {})", x, y) }
                    //                ).collect::<Vec<String>>().join(", "));
                } else {
                    if let Some(turn) = crate::gameplay::str_to_loc(&input) {
                        if valid_moves.contains(&Some(turn)) {
                            break Some(turn);
                        } else {
                            println!("Not a valid move!");
                            continue;
                        }
                    } else {
                        println!("Could not parse coordinate!");
                    }
                }
            }
        }
    }
}
