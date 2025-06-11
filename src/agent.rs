use std::cmp::Ordering;
use std::io;

use rand::prelude::IndexedRandom;
use rand::rngs::ThreadRng;

use crate::gameplay::{Gamestate, Turn, States, Players};

pub trait Agent {
    fn make_move(&mut self, state: &Gamestate) -> Turn;
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
    fn make_move(&mut self, state: &Gamestate) -> Turn {
        let valid_moves = state.get_moves();
        valid_moves.choose(&mut self.r)
                   .copied()
                   .expect("There were no moves because the game was over.")
    }
}

pub struct GreedyAgent {}

impl Agent for GreedyAgent {
    fn make_move(&mut self, state: &Gamestate) -> Turn {
        state.get_moves()
             .iter()
             .max_by(|t1, t2| -> Ordering {
                 // TODO: figure out wth derefing does to borrowing
                 let v1 = state.clone().make_move(**t1).expect("").len();
                 let v2 = state.clone().make_move(**t2).expect("").len();
                 v1.cmp(&v2)
             })
            .copied()
            .expect("Game was already won!")
    }
}

pub struct HumanAgent {}

impl Agent for HumanAgent {
    fn make_move(&mut self, state: &Gamestate) -> Turn {
        let stdin = io::stdin();
        let mut input = String::new();
        let valid_moves = state.get_moves();

        if valid_moves.is_empty() {
            panic!("Game is finished");
        } else {
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
}

pub struct MemoryHumanAgent {
    game: Gamestate,
}

impl MemoryHumanAgent {
    pub fn new() -> Self {
        MemoryHumanAgent { game: Gamestate::new() }
    }
}

impl MemoryAgent for MemoryHumanAgent {
    fn initialize_game(&mut self, state: Gamestate) {
        self.game = state;
    }

    fn make_move(&mut self) -> Turn {
        let mut a = HumanAgent {};
        //println!("{}", self.game);
        let turn = a.make_move(&self.game);
        self.game.make_move(turn);
        turn
    }

    fn opponent_move(&mut self, op: &Turn) {
        self.game.make_move(*op);
    }
}

pub struct HumanDebugger {}

impl Agent for HumanDebugger {
    fn make_move(&mut self, state: &Gamestate) -> Turn {
        let stdin = io::stdin();
        let mut input = String::new();
        let valid_moves = state.get_moves();

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

pub trait MemoryAgent {
    fn initialize_game(&mut self, state: Gamestate);
    fn opponent_move(&mut self, op: &Turn);
    fn make_move(&mut self) -> Turn;
}

pub fn play_memory_agents
<A1: MemoryAgent, A2: MemoryAgent>
(agent1: &mut A1, agent2: &mut A2) -> (i8, Vec<Turn>) {
    let mut history: Vec<Turn> = Vec::new();
    let mut game = Gamestate::new();
    agent1.initialize_game(game.clone());

    println!("{}", game);
    let first_move = agent1.make_move();
    history.push(first_move);
    if !game.make_move_fast(first_move) {
        panic!("illegal move");
    }

    agent2.initialize_game(game.clone());

    loop {
        let valid_moves = game.get_moves();
        if valid_moves.is_empty() {
            println!("Game over - score: {}", game.score());
            break (game.score(), history);
        }
        println!("\n{}", game);

        let player_move = match game.whose_turn() {
            States::Taken(Players::Black) => agent1.make_move(),
            States::Taken(Players::White) => agent2.make_move(),
            _ => panic!("game should not be over"),
        };
        if !game.make_move_fast(player_move) {
            panic!("illegal move");
        }
        match game.whose_turn() {
            States::Taken(Players::Black) => agent1.opponent_move(&player_move),
            States::Taken(Players::White) => agent2.opponent_move(&player_move),
            _ => (),
        };
    }
}

