mod mechanics;
pub mod gameplay;
pub mod agent;
pub mod mcst;

use crate::agent::Agent;
use crate::mcst::{SelectionPolicy, ExpansionPolicy, DecisionPolicy};

struct RandomSelection {}
struct RandomExpansion {}
struct SimpleDecision {}

fn main() {
    let t = mcst::McstTree::new(6);
    println!("{:?}", t);
}

fn play_a_game() {
    let mut g = crate::gameplay::Gamestate::new();

    let greedy = agent::GreedyAgent {};
    let human = agent::HumanDebugger {};

    loop {
        let valid_moves = g.get_moves();
        if valid_moves.is_empty() {
            println!("Game over - score: {}", g.score());
            break;
        }
        println!("{}", g);

        let player_move = match g.whose_turn() {
            crate::gameplay::Players::Black => human.make_move(&g),
            crate::gameplay::Players::White => greedy.make_move(&g),
        };
        g.make_turn(player_move.0, player_move.1);
    }
}
