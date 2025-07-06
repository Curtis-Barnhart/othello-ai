pub mod implementations;

use std::cmp::Ordering;

use crate::gameplay::{Gamestate, Turn, States, Players};

/// An Agent implements what is the bare minimum to play a game:
/// taking a look at a board and spitting out a valid turn.
pub trait Agent {
    fn make_move(&self, state: &Gamestate) -> Turn;
}

/// A MemoryAgent is a little more complicated than an [Agent].
/// Instead of just looking at a board and spitting out a move,
/// it provides the ability to carry information from previous turns
/// to future turns.
pub trait MemoryAgent {
    fn initialize_game(&mut self, state: Gamestate);
    fn opponent_move(&mut self, op: &Turn);
    fn make_move(&mut self) -> Turn;
}

pub struct MemorifiedAgent<A: Agent> { 
    memory: Gamestate,
    agent: A,
}

impl<A: Agent> MemorifiedAgent<A> {
    pub fn new(agent: A) -> Self {
        Self {
            memory: Gamestate::new(),
            agent,
        }
    }
}

impl<A: Agent> MemoryAgent for MemorifiedAgent<A> {
    fn initialize_game(&mut self, state: Gamestate) {
        self.memory = state;
    }

    fn opponent_move(&mut self, op: &Turn) {
        if !self.memory.make_move_fast(*op) {
            panic!("opponent_move passed invalid turn.");
        }
    }

    fn make_move(&mut self) -> Turn {
        let turn = self.agent.make_move(&self.memory);
        if !self.memory.make_move_fast(turn) {
            panic!("agent.make_move returned invalid turn.");
        }
        turn
    }
}

/// agent1 will always take the first turn from the current state,
/// regardless of if that turn is Black's or White's.
pub fn play_memory_agents_from
<A1: MemoryAgent, A2: MemoryAgent>
(agent_black: &mut A1, agent_white: &mut A2, mut game: Gamestate) -> (i8, Vec<Turn>) {
    let mut history: Vec<Turn> = Vec::new();
    let black_first = match game.whose_turn() {
        States::Empty => return (game.score(), Vec::new()),
        States::Taken(Players::Black) => true,
        States::Taken(Players::White) => false,
    };

    match black_first {
        true => {
            agent_black.initialize_game(game.clone());
            let first_move = agent_black.make_move();
            history.push(first_move);
            if !game.make_move_fast(first_move) {
                panic!("illegal move");
            }
            agent_white.initialize_game(game.clone());
        }
        false => {
            agent_white.initialize_game(game.clone());
            let first_move = agent_white.make_move();
            history.push(first_move);
            if !game.make_move_fast(first_move) {
                panic!("illegal move");
            }
            agent_black.initialize_game(game.clone());
        }
    }

    loop {
        let valid_moves = game.get_moves();
        if valid_moves.is_empty() {
            break (game.score(), history);
        }

        let player_move = match game.whose_turn() {
            States::Taken(Players::Black) => agent_black.make_move(),
            States::Taken(Players::White) => agent_white.make_move(),
            _ => panic!("game should not be over"),
        };
        if !game.make_move_fast(player_move) {
            panic!("illegal move {:?} on game \n{game}\n.", player_move);
        }
        history.push(player_move);
        match game.whose_turn() { // whose turn has just been updated
            States::Taken(Players::Black) => agent_black.opponent_move(&player_move),
            States::Taken(Players::White) => agent_white.opponent_move(&player_move),
            _ => (),
        };
    }
}

pub fn play_memory_agents
<A1: MemoryAgent, A2: MemoryAgent>
(agent1: &mut A1, agent2: &mut A2) -> (i8, Vec<Turn>) {
    play_memory_agents_from(agent1, agent2, Gamestate::new())
}

pub fn benchmark_memory_agents
<A1: MemoryAgent, A2: MemoryAgent>
(agent1: &mut A1, agent2: &mut A2, count: u32) -> f64 {
    let mut a1_score: f64 = 0_f64;
    for _ in 0..count {
        a1_score += match play_memory_agents(agent1, agent2).0.cmp(&0) {
            Ordering::Greater => 1_f64,
            Ordering::Less => 0_f64,
            _ => 0.5_f64,
        }
    }
    a1_score / f64::from(count)
}
