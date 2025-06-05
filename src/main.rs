mod mechanics;
pub mod gameplay;
pub mod agent;
pub mod mcst;

use std::cmp::Ordering;
use std::collections::VecDeque;
use std::io::{stdout, Write};

use mcst::McstAgent;

use crate::gameplay::Gamestate;
use crate::agent::{Agent, RandomAgent};
use crate::mcst::{SelectionPolicy, ExpansionPolicy, DecisionPolicy, McstNode, McstTree};

struct BfsSelectionFast {
    // queue of nodes to check which must already be in the tree
    queue: VecDeque<Vec<(u8, u8)>>,
}

impl BfsSelectionFast {
    pub fn new() -> Self {
        BfsSelectionFast {
            queue: VecDeque::from([Vec::new()]),
        }
    }
}

impl SelectionPolicy for BfsSelectionFast {
    fn select(&mut self, tree: &McstTree, game: &Gamestate) -> Option<Vec<(u8, u8)>> {
        loop {
            if let Some(path) = self.queue.pop_front() {
                let mut current_game = game.clone();
                current_game.make_turns(&path);
                let current_moves = current_game.get_moves();
                let current_moves_len = current_moves.len();

                if current_moves_len - tree.root.search(&path).unwrap().children.len() == 0 {
                    // we have already been here... put in the children and try again
                    for m in current_moves {
                        let mut next_path = path.clone();
                        next_path.push(m);
                        self.queue.push_back(next_path);
                    }
                } else {
                    self.queue.push_front(path.clone());
                    break Some(path);
                }
            } else {
                break Some(Vec::new());
            }
        }
    }

    fn turns_passed(&mut self, tree: &McstTree, game: &Gamestate, turns: ((u8, u8), (u8, u8))) {
        self.queue.clear();
        self.queue.push_back(Vec::new());
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
        panic!("No nodes to expand on given path {:?}", path);
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
    play_mcst();
}

fn run_mcst_agent(
    agent: &mut McstAgent<BfsSelectionFast, BfsExpansion, SimpleDecision, RandomAgent>,
    cycles: u32
) -> (u8, u8) {
    println!("");
    let mut percent = 0;
    for c in 0..cycles {
        match agent.cycle() {
            Ok(continuing) => {
                if !continuing {
                    panic!("quit");
                } else {
                    let temp_percent = (c * 100) / cycles;
                    if temp_percent > percent {
                        percent = temp_percent;
                        print!("\rcompleted {:03}%", percent);
                        let _ = stdout().flush();
                    }
                }
            }
            Err(e) => { panic!("errored on {:?}", e) },
        };
    }
    println!("\rcompleted 100%");

    if let Some(decision) = agent.decide() {
        println!("Best move: ({}, {})", decision.0, decision.1);
        let new_node = agent.view_tree().root.children.get(&decision).unwrap();
        println!("Win rate: {}/{}", new_node.wins, new_node.total);
        decision
    } else {
        panic!("no decision");
    }
}

fn play_mcst() {
    let mut g = Gamestate::new();

    let mut human = agent::HumanDebugger {};
    let cycles = 1 << 17;
    let mut mcst_agent = mcst::McstAgent::new(
        BfsSelectionFast::new(),
        BfsExpansion {},
        SimpleDecision {},
        RandomAgent::new(),
        RandomAgent::new(),
        cycles,
    );

    println!("{}", g);
    let first_move = human.make_move(&g);
    mcst_agent.game.make_turn(first_move.0, first_move.1);
    g.make_turn(first_move.0, first_move.1);

    let mut computer_move: (u8, u8) = (0, 0);

    loop {
        println!("{}", g);
        let valid_moves = g.get_moves();
        if valid_moves.is_empty() {
            println!("Game over - score: {}", g.score());
            break;
        }

        let player_move = match g.whose_turn() {
            crate::gameplay::Players::Black => human.make_move(&g),
            crate::gameplay::Players::White => run_mcst_agent(&mut mcst_agent, cycles),
        };
        match g.whose_turn() {
            crate::gameplay::Players::Black => {
                mcst_agent.next_two_moves(computer_move, player_move);
            }
            crate::gameplay::Players::White => { computer_move = player_move },
        }
        g.make_turn(player_move.0, player_move.1);
    }
}

fn play_a_game() {
    let mut g = Gamestate::new();

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
