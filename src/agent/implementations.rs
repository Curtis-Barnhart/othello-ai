use rand::prelude::IndexedRandom;
use rand::rngs::ThreadRng;
use std::time::Instant;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::io;

use crate::agent::{Agent, MemoryAgent};
use crate::gameplay::{Gamestate, Turn};
use crate::mcst::{McstNode, McstTree, McstAgent, SelectionPolicy, ExpansionPolicy, DecisionPolicy};

/// An agent that selects a random valid move each turn.
pub struct RandomAgent {
    r: RefCell<ThreadRng>,
}

impl RandomAgent {
    /// Constructs a new `RandomAgent` using thread-local RNG.
    pub fn new() -> Self {
        RandomAgent {r: RefCell::new(rand::rng())}
    }
}

impl Agent for RandomAgent {
    /// Chooses a random move from the list of valid moves.
    /// Will panic if there are no moves.
    fn make_move(&self, state: &Gamestate) -> Turn {
        let valid_moves = state.get_moves();
        valid_moves.choose(&mut self.r.borrow_mut())
                   .copied()
                   .expect("make_move passed state with no moves.")
    }
}

/// An agent that plays the move resulting in the most flips (greedy strategy).
pub struct GreedyAgent {}

impl Agent for GreedyAgent {
    /// Selects the move that flips the most opponent pieces.
    /// Panics if there are no valid moves.
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

/// A human-controlled agent.
pub struct MemoryHumanAgent {
    game: Gamestate,
}

impl MemoryHumanAgent {
    /// Constructs a new human agent with a fresh game state.
    pub fn new() -> Self {
        MemoryHumanAgent { game: Gamestate::new() }
    }
}

impl Agent for MemoryHumanAgent {
    /// Interacts with the user to input a valid move.
    /// Panics if there are no valid moves.
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
    /// Initializes internal game state with `state`.
    fn initialize_game(&mut self, state: Gamestate) {
        self.game = state;
    }

    /// Makes a move and updates the internal game state accordingly.
    fn make_move(&mut self) -> Turn {
        let turn = Agent::make_move(self, &self.game);
        self.game.make_move(turn);
        turn
    }

    /// Applies the opponent's move to the internal game state.
    fn opponent_move(&mut self, op: &Turn) {
        self.game.make_move(*op);
    }
}

/// A human agent for debugging and interactive play with command support.
pub struct HumanDebugger {}

impl Agent for HumanDebugger {
    /// Allows user to enter moves and execute debugging commands like `/moves` and `/history`.
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

// A UCT (Upper Confidence Bound applied to Trees) selection policy
pub struct UctSelection {
    /// Exploration constant.
    c: f64
}

impl UctSelection {
    /// Creates a new `UctSelection` with the specified exploration constant `c`.
    pub fn new(c: f64) -> Self {
        UctSelection { c: c }
    }

    /// Recursively selects nodes from the current player's perspective using UCT.
    /// Adds moves to the path until a node with no or unexplored children is reached.
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

    /// Recursively selects nodes from the opponent's perspective using inverted reward.
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
    /// Returns a path through the tree according to UCT-based selection.
    fn select(&mut self, tree: &McstTree) -> Option<Vec<Turn>> {
        let mut turns: Vec<Turn> = Vec::new();
        self.select_mine(tree.root(), &mut turns, tree.root().game().get_moves().len() as f64);
        Some(turns)
    }
}

/// A breadth-first search selection policy for MCTS.
/// Expands nodes level-by-level in the tree.
pub struct BfsSelectionFast {
    /// Queue of paths to nodes in the tree.
    queue: VecDeque<Vec<Turn>>,
}

impl BfsSelectionFast {
    /// Creates a new BFS selection policy initialized with the root node.
    pub fn new() -> Self {
        BfsSelectionFast {
            queue: VecDeque::from([Vec::new()]),
        }
    }
}

impl SelectionPolicy for BfsSelectionFast {
    /// Returns the next unexplored path according to BFS order.
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

    /// Resets the BFS queue at the start of a new turn.
    fn turns_passed(&mut self, _tree: &McstTree, _turns: (Turn, Turn)) {
        self.queue.clear();
        self.queue.push_back(Vec::new());
    }
}

/// A basic expansion policy that expands the first unvisited move.
pub struct BfsExpansion {}

impl ExpansionPolicy for BfsExpansion {
    /// Returns the first legal move from the given node that hasn't been expanded yet.
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

/// Decision policy that selects the move with the most simulations.
pub struct UctDecision {}

impl DecisionPolicy for UctDecision {
    /// Picks the move with the highest visit count from the root node.
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

/// Decision policy that selects the move with the best average win rate.
pub struct WinAverageDecision {}

impl DecisionPolicy for WinAverageDecision  {
    /// Picks the move with the highest win average (wins / total simulations).
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

/// An MCTS agent that uses BFS selection and win-average decision-making,
/// with memory of previous moves.
pub struct BfsMemoryAgent {
    agent: McstAgent<
        BfsSelectionFast,
        BfsExpansion,
        WinAverageDecision,
        RandomAgent,
    >,
    compute_time: u128,
    last_move: Turn,
}

impl BfsMemoryAgent {
    /// Creates a new BFS-based agent with the given computation time budget (in centiseconds).
    pub fn new(compute_time: u128) -> Self {
        BfsMemoryAgent {
            agent: McstAgent::new(
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

impl MemoryAgent for BfsMemoryAgent {
    /// Initializes the agent with a new game state.
    fn initialize_game(&mut self, state: Gamestate) {
        self.agent = McstAgent::new(
        BfsSelectionFast::new(),
        BfsExpansion {},
        WinAverageDecision {},
        RandomAgent::new(),
        RandomAgent::new(),
        state,
        )
    }

    /// Performs MCTS cycles until the time limit is reached,
    /// then returns the selected move.
    fn make_move(&mut self) -> Turn {
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

    /// Updates the internal game state with the opponent's move.
    fn opponent_move(&mut self, op: &Turn) {
        self.agent.next_two_moves(self.last_move, *op);
    }
}

/// An MCTS agent that uses UCT selection and total-count-based decision-making.
pub struct UctMemoryAgent {
    agent: McstAgent<
        UctSelection,
        BfsExpansion,
        UctDecision,
        RandomAgent,
    >,
    compute_time: u128,
    last_move: Turn,
}

impl UctMemoryAgent {
    /// Creates a new UCT-based agent with a given computation time and UCT exploration constant.
    pub fn new(compute_time: u128, learn_rate: f64) -> Self {
        UctMemoryAgent {
            agent: McstAgent::new(
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
    /// Initializes the agent with a new game state.
    fn initialize_game(&mut self, state: Gamestate) {
        self.agent = McstAgent::new(
        UctSelection::new(2_f64.sqrt()),
        BfsExpansion {},
        UctDecision {},
        RandomAgent::new(),
        RandomAgent::new(),
        state,
        )
    }

    /// Performs MCTS cycles until the time limit is reached,
    /// then returns the selected move.
    fn make_move(&mut self) -> Turn {
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

    /// Updates the internal game state with the opponent's move.
    fn opponent_move(&mut self, op: &Turn) {
        self.agent.next_two_moves(self.last_move, *op);
    }
}
