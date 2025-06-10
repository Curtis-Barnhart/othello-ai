mod mechanics;
pub mod gameplay;
pub mod agent;
pub mod mcst;

use std::cmp::Ordering;
use std::collections::VecDeque;
use std::io::{stdout, Write};
use std::time::Instant;

use mcst::McstAgent;

use crate::gameplay::{Gamestate, Turn, States, Players};
use crate::agent::{Agent, RandomAgent};
use crate::mcst::{SelectionPolicy, ExpansionPolicy, DecisionPolicy, McstTree};

struct BfsSelectionFast {
    // queue of nodes to check which must already be in the tree
    queue: VecDeque<Vec<Turn>>,
}

impl BfsSelectionFast {
    pub fn new() -> Self {
        BfsSelectionFast {
            queue: VecDeque::from([Vec::new()]),
        }
    }
}

impl SelectionPolicy for BfsSelectionFast {
    fn select(&mut self, tree: &McstTree, game: &Gamestate) -> Option<Vec<Turn>> {
        loop {
            if let Some(path) = self.queue.pop_front() {
                let current_moves = tree.root()
                                        .search(&path)
                                        .unwrap()
                                        .game()
                                        .get_moves();

                if !current_moves.is_empty() {
                    // there are moves to make
                    let move_ct = current_moves.len();
                    if move_ct - tree.root().search(&path).unwrap().children().len() == 0 {
                        // we have already been here... put in the children and try again
                        // TODO: also find out if there is a way that doesn't need &*
                        for m in &*current_moves {
                            let mut next_path = path.clone();
                            next_path.push(*m);
                            self.queue.push_back(next_path);
                        }
                    } else {
                        self.queue.push_front(path.clone());
                        break Some(path);
                    }
                } // else game is over and cannot be selected
            } else {
                break None;
            }
        }
    }

    fn turns_passed(&mut self, tree: &McstTree, game: &Gamestate, turns: (Turn, Turn)) {
        self.queue.clear();
        self.queue.push_back(Vec::new());
    }
}

struct BfsExpansion {}

impl ExpansionPolicy for BfsExpansion {
    fn expand(&mut self, tree: &McstTree, path: &Vec<Turn>, game: &Gamestate) -> Turn {
        let node = tree.root().search(&path).unwrap();
        for next_turn in &*node.game().get_moves() {
            if !node.children().contains_key(&next_turn) {
                return *next_turn;
            }
        }
        panic!("No nodes to expand on given path {:?}", path);
    }
}

struct SimpleDecision {}

impl DecisionPolicy for SimpleDecision {
    fn decide(&mut self, tree: &McstTree, game: &Gamestate) -> Turn {
        tree.root().children().keys().max_by(
            |link1, link2| -> Ordering {
                let node1 = tree.root().children().get(link1).unwrap();
                let node2 = tree.root().children().get(link2).unwrap();
                match (node1.wins(), node1.total(), node2.wins(), node2.total()) {
                    (_, 0, _, 0) => Ordering::Equal,
                    (_, 0, _, _) => Ordering::Less,
                    (_, _, _, 0) => Ordering::Greater,
                    (w1, t1, w2, t2) => 
                        (f64::from(*w1) / f64::from(*t1)).total_cmp(&(f64::from(*w2) / f64::from(*t2)))
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
    compute_time: u128,
) -> Turn {
    println!("");
    let time_0 = Instant::now();
    let mut hundreths: u128 = 0;
    let mut count: u32 = 0;
    loop {
        match agent.cycle() {
            Ok(continuing) => {
                if !continuing {
                    break;
                } else {
                    count += 1;
                    let delta = time_0.elapsed().as_millis() / 10;
                    if delta >= hundreths {
                        hundreths = delta;
                        print!("\relapsed seconds: {}.{:02}", hundreths / 100, hundreths % 100);
                        let _ = stdout().flush();
                        if hundreths > compute_time {
                            break;
                        }
                    }
                }
            }
            Err(e) => { panic!("errored on {:?}", e) },
        };
    }
    if hundreths == 0 {
        println!("\nCompleted {} iterations (NaN/sec)", count);
    } else {
        println!("\nCompleted {} iterations ({}/sec)", count, u128::from(count * 100) / hundreths);
    }

    let decision = match agent.decide() {
        Some(Some(loc)) => {
            println!("Best move: ({}, {})", loc.0, loc.1);
            Some(loc)
        },
        Some(Option::None) => {
            println!("Best move: pass");
            None
        }
        _ => panic!("Decision could not be made"),
    };
    let new_node = agent.view_tree().root().children().get(&decision).unwrap();
    println!("Win rate: {}/{}", new_node.wins(), new_node.total());
    decision
}

fn play_mcst() {
    let mut g = Gamestate::new();
    let mut human = agent::HumanDebugger {};
    let second_hundreths = 300;

    println!("{}", g);
    let first_move = human.make_move(&g);
    g.make_move_fast(first_move);

    let mut mcst_agent = mcst::McstAgent::new(
        BfsSelectionFast::new(),
        BfsExpansion {},
        SimpleDecision {},
        RandomAgent::new(),
        RandomAgent::new(),
        g.clone(),
    );

    let mut computer_move: Turn = None;

    loop {
        println!("{}", g);
        let valid_moves = g.get_moves();
        if valid_moves.is_empty() {
            println!("Game over - score: {}", g.score());
            break;
        }

        let player_move = match g.whose_turn() {
            States::Taken(Players::Black) => human.make_move(&g),
            States::Taken(Players::White) => run_mcst_agent(&mut mcst_agent, second_hundreths),
            _ => panic!("game should not be over"),
        };
        match g.whose_turn() {
            States::Taken(Players::Black) => {
                mcst_agent.next_two_moves(computer_move, player_move);
            }
            States::Taken(Players::White) => { computer_move = player_move },
            _ => panic!("game should not be over"),
        }
        g.make_move_fast(player_move);
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
            States::Taken(Players::Black) => human.make_move(&g),
            States::Taken(Players::White) => greedy.make_move(&g),
            _ => panic!("game should not be over"),
        };
        g.make_move(player_move);
    }
}
