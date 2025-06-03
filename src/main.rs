mod mechanics;
pub mod gameplay;
pub mod agent;
pub mod mcst;

use std::cmp::Ordering;
use std::collections::VecDeque;

use mcst::CycleError;

use crate::gameplay::Gamestate;
use crate::agent::{Agent, RandomAgent};
use crate::mcst::{SelectionPolicy, ExpansionPolicy, DecisionPolicy, McstNode, McstTree};

struct BfsSelection {}

impl SelectionPolicy for BfsSelection {
    fn select(&mut self, tree: &McstTree, game: &Gamestate) -> Option<Vec<(u8, u8)>> {
        let mut queue = VecDeque::<(&McstNode, Vec<(u8, u8)>)>::new();
        queue.push_back((&tree.root, Vec::new()));
        'node_loop:
        loop {
            if let Some((node, path)) = queue.pop_front() {
                let mut new_game = game.clone();
                if !new_game.make_turns(&path) {
                    panic!("Couldn't make the turns to a node we put in the queue earlier");
                }
                for turn in new_game.get_moves() {
                    if node.children.contains_key(&turn) {
                        let mut new_path = path.clone();
                        new_path.push(turn);
                        queue.push_back((
                            node.children.get(&turn).expect("we checked last line"),
                            new_path,
                        ));
                    } else {
                        break 'node_loop Some(path);
                    }
                }
            } else {
                break None;
            }
        }
    }
}

struct BfsExpansion {}

impl ExpansionPolicy for BfsExpansion {
    fn expand(&mut self, tree: &McstTree, path: &Vec<(u8, u8)>, game: &Gamestate) -> (u8, u8) {
        let mut new_game = game.clone();
        new_game.make_turns(path);
        for next_turn in new_game.get_moves() {
            if !tree.root
                    .search(path)
                    .expect("Invalid path given for expansion")
                    .children
                    .contains_key(&next_turn) {
                return next_turn;
            }
        }
        panic!("No nodes to expand on given path");
    }
}

struct SimpleDecision {}

impl DecisionPolicy for SimpleDecision {
    fn decide(&mut self, tree: &McstTree, game: &Gamestate) -> (u8, u8) {
        tree.root.children.keys().max_by(
            |link1, link2| -> Ordering {
                let node1 = tree.root.children.get(link1).unwrap();
                let node2 = tree.root.children.get(link2).unwrap();
                match (node1.wins, node1.total, node2.wins, node2.total) {
                    (_, 0, _, 0) => Ordering::Equal,
                    (_, 0, _, _) => Ordering::Less,
                    (_, _, _, 0) => Ordering::Greater,
                    (w1, t1, w2, t2) => 
                        (f64::from(w1) / f64::from(t1)).total_cmp(&(f64::from(w2) / f64::from(t2)))
                }
            }
        ).copied().expect("Somehow there no moves?")
    }
}

fn main() {
    let mut agent = mcst::McstAgent::new(
        BfsSelection {},
        BfsExpansion {},
        SimpleDecision {},
        RandomAgent::new(),
        RandomAgent::new(),
        4096,
    );

    for c in 0..4096 {
        match agent.cycle() {
            Ok(continuing) => {
                if !continuing {
                    panic!("quit");
                } else {
                    println!("completed {}", c);
                }
            }
            Err(e) => { panic!("errored on {:?}", e) },
        };
    }

    if let Some(decision) = agent.decide() {
        println!("Best move: ({}, {})", decision.0, decision.1);
        let new_node = agent.view_tree().root.children.get(&decision).unwrap();
        println!("Win rate: {}/{}", new_node.wins, new_node.total);
    } else {
        panic!("no decision");
    }
}

fn play_a_game() {
    let mut g = crate::gameplay::Gamestate::new();

    let mut greedy = agent::GreedyAgent {};
    let mut human = agent::HumanDebugger {};

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
