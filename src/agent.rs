pub mod implementations;

use crate::gameplay::{Gamestate, Turn, States, Players};

pub trait Agent {
    fn make_move(&self, state: &Gamestate) -> Turn;
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

    let first_move = agent1.make_move();
    history.push(first_move);
    if !game.make_move_fast(first_move) {
        panic!("illegal move");
    }

    agent2.initialize_game(game.clone());

    loop {
        let valid_moves = game.get_moves();
        if valid_moves.is_empty() {
            break (game.score(), history);
        }

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

