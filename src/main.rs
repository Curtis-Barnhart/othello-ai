mod mechanics;
pub mod gameplay;
pub mod agent;
pub mod mcst;

use std::cmp::Ordering;
use std::collections::VecDeque;
use std::io::{stdout, stdin, Write};
use std::time::Instant;
use std::env;

use mechanics::Board;
use crate::gameplay::{Gamestate, Turn, States, Players};
use agent::{play_memory_agents, MemoryAgent};
use agent::implementations::RandomAgent;
use crate::mcst::{SelectionPolicy, ExpansionPolicy, DecisionPolicy, McstTree, McstAgent, McstNode};

struct UctSelection {
    c: f64
}

impl UctSelection {
    pub fn new(c: f64) -> Self {
        UctSelection { c: c }
    }

    fn select_mine(&self, node: &McstNode, path: &mut Vec<Turn>, total: f64) {
        if node.children().len() < node.game().get_moves().len()
           || node.children().len() == 0 {
        } else {
            let new_child = node.children().iter().max_by(
                |n1, n2| -> Ordering {
                    let n1w = f64::from(*n1.1.wins());
                    let n1t = f64::from(*n1.1.total());
                    let n2w = f64::from(*n2.1.wins());
                    let n2t = f64::from(*n2.1.total());
                    (n1w / n1t + self.c * (total.ln() / n1w).sqrt()).total_cmp(
                        &(n2w / n2t + self.c * (total.ln() / n2w).sqrt())
                    )
                }
            ).expect("There were no children?");
            path.push(*new_child.0);
            self.select_your(new_child.1, path, f64::from(*new_child.1.total()));
        }
    }

    fn select_your(&self, node: &McstNode, path: &mut Vec<Turn>, total: f64) {
        if node.children().len() < node.game().get_moves().len()
           || node.children().len() == 0 {
        } else {
            let new_child = node.children().iter().max_by(
                |n1, n2| -> Ordering {
                    let n1w = f64::from(*n1.1.wins());
                    let n1t = f64::from(*n1.1.total());
                    let n2w = f64::from(*n2.1.wins());
                    let n2t = f64::from(*n2.1.total());
                    (-n1w / n1t + self.c * (total.ln() / n1w).sqrt()).total_cmp(
                        &(-n2w / n2t + self.c * (total.ln() / n2w).sqrt())
                    )
                }
            ).expect("There were no children?");
            path.push(*new_child.0);
            self.select_mine(new_child.1, path, f64::from(*new_child.1.total()));
        }
    }
}

impl SelectionPolicy for UctSelection {
    fn select(&mut self, tree: &McstTree) -> Option<Vec<Turn>> {
        let mut turns: Vec<Turn> = Vec::new();
        self.select_mine(tree.root(), &mut turns, tree.root().game().get_moves().len() as f64);
        Some(turns)
    }
}

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
    fn select(&mut self, tree: &McstTree) -> Option<Vec<Turn>> {
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

    fn turns_passed(&mut self, _tree: &McstTree, _game: &Gamestate, _turns: (Turn, Turn)) {
        self.queue.clear();
        self.queue.push_back(Vec::new());
    }
}

struct BfsExpansion {}

impl ExpansionPolicy for BfsExpansion {
    fn expand(&mut self, tree: &McstTree, path: &Vec<Turn>) -> Turn {
        let node = tree.root().search(&path).unwrap();
        for next_turn in &*node.game().get_moves() {
            if !node.children().contains_key(&next_turn) {
                return *next_turn;
            }
        }
        panic!("No nodes to expand on given path {:?}", path);
    }
}

struct UctDecision {}

impl DecisionPolicy for UctDecision {
    fn decide(&mut self, tree: &McstTree) -> Turn {
        tree.root().children().keys().max_by(
            |link1, link2| -> Ordering {
                let node1 = tree.root().children().get(link1).unwrap();
                let node2 = tree.root().children().get(link2).unwrap();
                node1.total().cmp(node2.total())
            }
        ).copied().expect("Somehow there no moves?")
    }
}

struct WinAverageDecision {}

impl DecisionPolicy for WinAverageDecision  {
    fn decide(&mut self, tree: &McstTree) -> Turn {
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

struct UctMemoryAgent {
    agent: mcst::McstAgent<
        UctSelection,
        BfsExpansion,
        UctDecision,
        RandomAgent,
    >,
    compute_time: u128,
    last_move: Turn,
}

struct BfsMemoryAgent {
    agent: mcst::McstAgent<
        BfsSelectionFast,
        BfsExpansion,
        WinAverageDecision,
        RandomAgent,
    >,
    compute_time: u128,
    last_move: Turn,
}

impl BfsMemoryAgent {
    pub fn new(compute_time: u128) -> Self {
        BfsMemoryAgent {
            agent: mcst::McstAgent::new(
            BfsSelectionFast::new(),
            BfsExpansion {},
            WinAverageDecision {},
            RandomAgent::new(),
            RandomAgent::new(),
            Gamestate::new(),
            ),
            compute_time: compute_time,
            last_move: None,
        }
    }
}

impl UctMemoryAgent {
    pub fn new(compute_time: u128, learn_rate: f64) -> Self {
        UctMemoryAgent {
            agent: mcst::McstAgent::new(
            UctSelection::new(learn_rate),
            BfsExpansion {},
            UctDecision {},
            RandomAgent::new(),
            RandomAgent::new(),
            Gamestate::new(),
            ),
            compute_time: compute_time,
            last_move: None,
        }
    }
}

impl MemoryAgent for UctMemoryAgent {
    fn initialize_game(&mut self, state: Gamestate) {
        self.agent = mcst::McstAgent::new(
        UctSelection::new(2_f64.sqrt()),
        BfsExpansion {},
        UctDecision {},
        RandomAgent::new(),
        RandomAgent::new(),
        state,
        )
    }

    fn make_move(&mut self) -> Turn {
        //println!("");
        let time_0 = Instant::now();
        let mut hundreths: u128 = 0;
        loop {
            match self.agent.cycle() {
                Ok(continuing) => {
                    if !continuing {
                        break;
                    } else {
                        let delta = time_0.elapsed().as_millis() / 10;
                        if delta >= hundreths {
                            hundreths = delta;
                            if hundreths > self.compute_time {
                                break;
                            }
                        }
                    }
                }
                Err(e) => { panic!("errored on {:?}", e) },
            };
        }

        let decision = match self.agent.decide() {
            Some(Some(loc)) => {
                Some(loc)
            },
            Some(Option::None) => {
                None
            }
            _ => panic!("Decision could not be made"),
        };

        self.last_move = decision;
        decision
    }

    fn opponent_move(&mut self, op: &Turn) {
        self.agent.next_two_moves(self.last_move, *op);
    }
}

impl MemoryAgent for BfsMemoryAgent {
    fn initialize_game(&mut self, state: Gamestate) {
        self.agent = mcst::McstAgent::new(
        BfsSelectionFast::new(),
        BfsExpansion {},
        WinAverageDecision {},
        RandomAgent::new(),
        RandomAgent::new(),
        state,
        )
    }

    fn make_move(&mut self) -> Turn {
        //println!("");
        let time_0 = Instant::now();
        let mut hundreths: u128 = 0;
        loop {
            match self.agent.cycle() {
                Ok(continuing) => {
                    if !continuing {
                        break;
                    } else {
                        let delta = time_0.elapsed().as_millis() / 10;
                        if delta >= hundreths {
                            hundreths = delta;
                            if hundreths > self.compute_time {
                                break;
                            }
                        }
                    }
                }
                Err(e) => { panic!("errored on {:?}", e) },
            };
        }

        let decision = match self.agent.decide() {
            Some(Some(loc)) => {
                Some(loc)
            },
            Some(Option::None) => {
                None
            }
            _ => panic!("Decision could not be made"),
        };

        self.last_move = decision;
        decision
    }

    fn opponent_move(&mut self, op: &Turn) {
        self.agent.next_two_moves(self.last_move, *op);
    }
}

fn main() {
    //let _ = stdin().read_line(&mut String::new());
    // from sqrt(2)/2 to 2sqrt(2)
    let base = 2_f64.sqrt() / 2_f64;
    let unit = base / 16_f64;

    loop {
        for m in 0..48 {
            let lr = base + f64::from(m) * unit;

            {
                let mut bfs = BfsMemoryAgent::new(100);
                let mut utc = UctMemoryAgent::new(100, lr);
                let (score, _) = play_memory_agents(&mut bfs, &mut utc);
                match score.partial_cmp(&0) {
                    Some(Ordering::Greater) => println!("second,{},loss", lr),
                    Some(Ordering::Less) => println!("second,{},win", lr),
                    Some(Ordering::Equal) => println!("second,{},tie", lr),
                    _ => panic!("wtf"),
                }
            }

            {
                let mut bfs = BfsMemoryAgent::new(100);
                let mut utc = UctMemoryAgent::new(100, lr);
                let (score, _) = play_memory_agents(&mut utc, &mut bfs);
                match score.partial_cmp(&0) {
                    Some(Ordering::Greater) => println!("first,{},win", lr),
                    Some(Ordering::Less) => println!("first,{},loss", lr),
                    Some(Ordering::Equal) => println!("first,{},tie", lr),
                    _ => panic!("wtf"),
                }
            }
        }
    }
}

fn test() {
    let mut b = Board::new();
    b.change(4, 3, States::Taken(Players::Black));
    b.change(2, 2, States::Taken(Players::Black));
    b.change(3, 2, States::Taken(Players::White));
    b.change(4, 2, States::Taken(Players::White));
    b.change(5, 2, States::Taken(Players::White));

    let g = Gamestate::new_mock(b, 0);

    let mut mcst_agent = mcst::McstAgent::new(
        UctSelection::new(2_f64.sqrt()),
        BfsExpansion {},
        UctDecision {},
        RandomAgent::new(),
        RandomAgent::new(),
        g.clone(),
    );

    for _ in 0..500 {
        let _ = mcst_agent.cycle();
    }

    for pair in mcst_agent.view_tree().root().children() {
        println!(
            "move ({}, {}) wins {} out of {} times",
            pair.0.unwrap().0,
            pair.0.unwrap().1,
            pair.1.wins(),
            pair.1.total(),
        );
    }
}
