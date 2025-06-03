use std::cmp::Ordering;
use std::io;

use rand::prelude::IndexedRandom;
use rand::rngs::ThreadRng;

use crate::gameplay;

pub trait Agent {
    fn make_move(&mut self, state: &gameplay::Gamestate) -> (u8, u8);
}

pub struct RandomAgent {
    r: ThreadRng,
}

impl RandomAgent {
    pub fn new() -> Self {
        RandomAgent {r: rand::rng()}
    }
}

impl Agent for RandomAgent {
    fn make_move(&mut self, state: &gameplay::Gamestate) -> (u8, u8) {
        state.get_moves()
            .choose(&mut self.r)
            .copied()
            .expect("There were no valid moves.")
    }
}

pub struct GreedyAgent {}

impl Agent for GreedyAgent {
    fn make_move(&mut self, state: &gameplay::Gamestate) -> (u8, u8) {
        state.get_moves()
            .iter()
            .max_by(|(x1, y1), (x2, y2)| -> Ordering {
                // TODO: figure out wth derefing does to borrowing
                let v1 = state.clone().make_turn(*x1, *y1).len();
                let v2 = state.clone().make_turn(*x2, *y2).len();
                v1.cmp(&v2)
            })
        .copied()
            .expect("There were no valid moves.")

    }
}

pub struct HumanAgent {}

impl Agent for HumanAgent {
    fn make_move(&mut self, state: &gameplay::Gamestate) -> (u8, u8) {
        let stdin = io::stdin();
        let mut input = String::new();
        let valid_moves = state.get_moves();

        loop {
            println!("Enter a coordinate:");
            input.clear();
            stdin.read_line(&mut input).expect("stdio could not be read from");
            input.pop();

            if let Some((x, y)) = crate::gameplay::str_to_loc(&input) {
                if valid_moves.contains(&(x, y)) {
                    break (x, y)
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

pub struct HumanDebugger {}

impl Agent for HumanDebugger {
    fn make_move(&mut self, state: &gameplay::Gamestate) -> (u8, u8) {
        let stdin = io::stdin();
        let mut input = String::new();
        let valid_moves = state.get_moves();

        loop {
            println!("Enter a coordinate:");
            input.clear();
            stdin.read_line(&mut input).expect("stdio could not be read from");
            input.pop();

            if input == "/moves" {
                println!("{}", valid_moves.iter().map(
                        |(x, y)| -> String { format!("({}, {})", x, y) }
                ).collect::<Vec<String>>().join(", "));
            } else if input == "/history" {
                println!("{}", state.view_history().iter().map(
                        |(x, y)| -> String { format!("({}, {})", x, y) }
                ).collect::<Vec<String>>().join(", "));
            } else {
                if let Some((x, y)) = crate::gameplay::str_to_loc(&input) {
                    if valid_moves.contains(&(x, y)) {
                        break (x, y)
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
