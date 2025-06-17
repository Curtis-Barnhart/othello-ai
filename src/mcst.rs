use std::collections::HashMap;
use std::cmp::Ordering;
use std::time::{Duration, Instant};

use crate::agent::Agent;
use crate::gameplay::{Gamestate, Players, States, Turn};

/// A trait for defining how nodes are selected during MCTS traversal.
pub trait SelectionPolicy {
    /// Select a path through the tree to expand or evaluate.
    fn select(&mut self, tree: &McstTree) -> Option<Vec<Turn>>;
    /// Inform the selector of the two most recent moves.
    fn turns_passed(&mut self, tree: &McstTree, turns: (Turn, Turn)) {}
}

/// A trait for defining how the tree expands new nodes.
pub trait ExpansionPolicy {
    /// Choose which move to expand from the given path.
    fn expand(&mut self, tree: &McstTree, path: &Vec<Turn>) -> Turn;
}

/// A trait for deciding which move to make from the current root state.
pub trait DecisionPolicy {
    /// Choose the best move to play based on the tree.
    fn decide(&mut self, tree: &McstTree) -> Turn;
}

/// A single node in the Monte Carlo Search Tree.
pub struct McstNode {
    /// The children of this node by which turn you take to get there.
    children: HashMap<Turn, McstNode>,
    /// How many wins rollouts from this node or its descendants have.
    wins: u32,
    /// How many rollouts from this node or its descendants have been played.
    total: u32,
    /// Gamestate at this node.
    game: Gamestate,
}

impl McstNode {
    /// Create a new node with the given game state.
    fn new(game: Gamestate) -> Self {
        McstNode {
            children: HashMap::new(),
            wins: 0,
            total: 0,
            game: game
        }
    }

    /// Immutable [McstNode::game] getter.
    pub fn game(&self) -> &Gamestate {
        &self.game
    }

    /// Immutable [McstNode::wins] getter.
    pub fn wins(&self) -> &u32 {
        &self.wins
    }

    /// Immutable [McstNode::total] getter.
    pub fn total(&self) -> &u32 {
        &self.total
    }

    pub fn node_count(&self) -> usize {
        if self.children.is_empty() {
            return 1
        } else {
            let mut c = 1;
            for (_, child) in &self.children {
                c += child.node_count();
            }
            c
        }
    }

    /// Immutable [McstNode::children] getter.
    pub fn children(&self) -> &HashMap<Turn, McstNode> {
        &self.children
    }

    /// Update the win count after a rollout.
    fn update(&mut self, win: bool) {
        if win { self.wins += 1 };
        self.total += 1;
    }

    /// Recursively search for a mutable reference to a node along a path.
    fn search_mut(&mut self, path: &[Turn]) -> Option<&mut McstNode> {
        if let Some(child) = &path.first() {
            if let Some(child) = self.children.get_mut(child) {
                child.search_mut(&path[1..])
            } else { None }
        } else { Some(self) }
    }

    /// Recursively search for an immutable reference to a node along a path.
    pub fn search(&self, path: &[Turn]) -> Option<&McstNode> {
        if let Some(child) = &path.first() {
            if let Some(child) = self.children.get(child) {
                child.search(&path[1..])
            } else { None }
        } else { Some(&self) }
    }
}

/// The Monte Carlo Search Tree.
pub struct McstTree {
    root: McstNode,
}

impl McstTree {
    /// Create a new MCTS tree from a game state.
    pub fn new(game: Gamestate) -> Self {
        McstTree {
            root: McstNode::new(game),
        }
    }

    /// Immutable [McstTree::root] getter.
    pub fn root(&self) -> &McstNode {
        &self.root
    }

    /// Add a child node by performing a move from a given path.
    ///
    /// # Panics
    /// If the path is invalid or the child already exists.
    pub fn add_child(&mut self, path: &[Turn], link: Turn) {
        if let Some(old) = self.root.search_mut(path) {
            if old.children.contains_key(&link) {
                panic!("already contained child");
            } else {
                let mut new_game = old.game.clone();
                if !new_game.make_move_fast(link) {
                    panic!("child didn't make real move");
                }
                let new_child = McstNode::new(new_game);
                old.children.insert(link, new_child);
            }
        } else {
            panic!("path was not valid");
        }
    }
}

/// Errors that can occur during a full MCTS cycle.
#[derive(Debug)]
pub enum CycleError {
    Selection(SelectionError),
    Expansion(ExpansionError),
    Rollout(RolloutError),
}

/// Errors that can occur during the selection phase.
#[derive(Debug)]
pub enum SelectionError {
    /// The path given by the selection policy does not lead to a valid node.
    NotANode(Vec<Turn>),
}

/// Errors that can occur during the expansion phase.
#[derive(Debug)]
pub enum ExpansionError {
    /// The selected move is illegal in the given game state.
    IllegalMove(Turn),
    /// A child node for the move already exists.
    AlreadyExpanded(Turn),
}

/// Errors that can occur during the rollout (simulation) phase.
#[derive(Debug)]
pub enum RolloutError {
    /// A move attempted during simulation was invalid.
    IllegalMove(Vec<Turn>),
}

/// A configurable MCTS agent composed of modular policies for selection,
/// expansion, rollout, and decision making.
pub struct McstAgent<
    S: SelectionPolicy,
    E: ExpansionPolicy,
    D: DecisionPolicy,
    R: Agent,
> {
    selector: S,
    expander: E,
    rollout: R,
    opponent: R,
    decider: D,
    tree: McstTree,
}

impl<
    S: SelectionPolicy,
    E: ExpansionPolicy,
    D: DecisionPolicy,
    R: Agent,
> McstAgent<S, E, D, R> {
    /// Construct a new MCTS agent using the given policies and starting state.
    pub fn new(
        selector: S,
        expander: E,
        decider: D,
        rollout: R,
        opponent: R,
        game: Gamestate,
    ) -> Self {
        McstAgent {
            selector: selector,
            expander: expander,
            decider: decider,
            rollout: rollout,
            opponent: opponent,
            tree: McstTree::new(game),
        }
    }

    /// Immutable [McstAgent::tree] getter.
    pub fn tree(&self) -> &McstTree {
        &self.tree
    }

    /// Run the selection phase.
    ///
    /// Returns a path iff a node was selected.
    /// Returns Ok(None) if the selector has decided there is no need to
    /// consider more cycles.
    /// Returns an error if the selector gave an invalid path.
    fn select(&mut self) -> Result<Option<Vec<Turn>>, SelectionError> {
        if let Some(path) = self.selector.select(&self.tree) {
            if let Some(_) = &self.tree.root.search(&path) {
                Ok(Some(path))
            } else { Err(SelectionError::NotANode(path)) }
        } else { Ok(None) }
    }

    /// Expand a new move from the node at the given path.
    ///
    /// # Panics
    /// If the path to the node to expand is invalid.
    fn expand(&mut self, path: &Vec<Turn>) -> Result<Turn, ExpansionError> {
        let link = self.expander.expand(&self.tree, path);
        let node = self.node_from_path(path); // may panic
        if node.game().get_moves().contains(&link) {
            if node.children.contains_key(&link) {
                Err(ExpansionError::AlreadyExpanded(link))
            } else {
                Ok(link)
            }
        } else {
            Err(ExpansionError::IllegalMove(link))
        }
    }

    /// Perform a simulated playout from the given path and
    /// return whether the root player won.
    ///
    /// # Panics
    /// On invalid `path`.
    fn rollout(&mut self, path: &Vec<Turn>, mut my_turn: bool) -> Result<bool, RolloutError> {
        let mut game = self.node_from_path(path).game().clone(); // panics on invalid path
        // TODO: optimize by removing move_history?
        let mut move_history: Vec<Turn> = Vec::new();
        let my_color = match self.tree.root.game.whose_turn() {
            States::Taken(c) => c,
            States::Empty => panic!("initial game is over?"),
        };

        loop {
            if !game.is_terminal() {
                let player_move = if my_turn {
                    self.rollout.make_move(&game)
                } else {
                    self.opponent.make_move(&game)
                };
                move_history.push(player_move);

                if !game.make_move_fast(player_move) {
                    break Err(RolloutError::IllegalMove(move_history));
                }
                my_turn = !my_turn;
            } else {
                break Ok(match (my_color, game.score().cmp(&0)) {
                    (Players::Black, Ordering::Greater) => true,
                    (Players::White, Ordering::Less) => true,
                    _ => false,
                });
            }
        }
    }

    /// Perform one full MCTS cycle: selection, expansion, rollout, backpropagation.
    ///
    /// Returns `Ok(false)` if the selector chose not to proceed
    /// and `Ok(true)` if it was successful and wants to continue cycling.
    pub fn cycle(&mut self) -> Result<bool, CycleError> {
        let path = self.select();
        let mut path = match path {
            Err(e) => return Err(CycleError::Selection(e)),
            Ok(Some(path)) => path,
            Ok(Option::None) => return Ok(false),
        };

        if !self.node_from_path(&path).game.is_terminal() {
            match self.expand(&path) { // won't panic because path is validated above
                Err(e) => return Err(CycleError::Expansion(e)),
                Ok(expansion) => {
                    self.tree.add_child(&path, expansion);
                    path.push(expansion);
                },
            };
        }

        let win = match self.rollout(&path, path.len() & 1 == 0) {
            Err(e) => return Err(CycleError::Rollout(e)),
            Ok(win) => win,
        };

        for index in 0..=path.len() {
            self.node_from_path_mut(&path[..index])
                .update(win);
        }

        Ok(true)
    }

    /// Choose a move to play based on the current tree.
    ///
    /// Returns `None` if the decision is invalid in the root game state.
    pub fn decide(&mut self) -> Option<Turn> {
        let decision = self.decider.decide(&self.tree);
        if self.tree.root.game.valid_move(decision) {
            Some(decision)
        } else {
            None
        }
    }

    /// Get a mutable reference to a node at a specific path.
    ///
    /// # Panics
    /// If the path does not refer to a valid node.
    fn node_from_path_mut(&mut self, path: &[Turn]) -> &mut McstNode {
        self.tree
            .root
            .search_mut(path)
            .expect("Node from path given invalid path")
    }

    /// Get an immutable reference to a node at a specific path.
    ///
    /// # Panics
    /// If the path does not refer to a valid node.
    fn node_from_path(&self, path: &[Turn]) -> &McstNode {
        self.tree
            .root
            .search(path)
            .expect("Node from path given invalid path")
    }

    /// Advance the tree to reflect two new moves.
    ///
    /// Replaces the root with the subtree corresponding to the new state.
    /// Returns `false` if the moves were invalid.
    pub fn next_two_moves(&mut self, mv1: Turn, mv2: Turn) -> bool {
        let mut test_game = self.tree.root.game.clone();
        if !test_game.make_moves_fast(&[mv1, mv2]) {
            false
        } else {
            // add first and second children if not in tree, then replace root
            if !self.tree.root.children.contains_key(&mv1) {
                // won't panic since it is verified that mv1 is not in children
                self.tree.add_child(&[], mv1);
            }
            // won't panic because we just put mv1 into the tree
            if !self.tree.root.children.get(&mv1).unwrap().children.contains_key(&mv2) {
                // won't panic since it is verified that mv2 is not in children
                self.tree.add_child(&[mv1], mv2); // panics on invalid path
            }
            // won't panic because we just put mv1 and mv2 into the tree
            self.tree.root = self.tree
                                 .root
                                 .children
                                 .get_mut(&mv1)
                                 .unwrap()
                                 .children
                                 .remove(&mv2)
                                 .unwrap();

            self.selector.turns_passed(&self.tree,  (mv1, mv2));
            true
        }
    }
}

/// Benchmarks an MCTS agent by running cycles for 5 seconds and
/// returnind the average number of nodes generated per second.
pub fn benchmark<Sel, Exp, Dec, Roll>(
    mut agent: McstAgent<Sel, Exp, Dec, Roll>,
) -> usize
where
    Sel: SelectionPolicy,
    Exp: ExpansionPolicy,
    Dec: DecisionPolicy,
    Roll: Agent,
{
    let start_time = Instant::now();
    let time_limit = Duration::from_secs(5);

    // Run as many cycles as possible within the time limit
    while Instant::now() - start_time < time_limit {
        if let Err(e) = agent.cycle() {
            panic!("Cycle failed during benchmarking: {:?}", e);
        }
    }

    let total_nodes = agent.tree().root().node_count();
    let elapsed_secs = (Instant::now() - start_time).as_secs_f64();

    (total_nodes as f64 / elapsed_secs).round() as usize
}
