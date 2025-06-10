use std::collections::HashMap;
use std::cmp::Ordering;

use crate::agent::Agent;
use crate::gameplay::{Gamestate, Players, States, Turn};

pub trait SelectionPolicy {
    fn select(&mut self, tree: &McstTree, game: &Gamestate) -> Option<Vec<Turn>>;
    fn turns_passed(&mut self, tree: &McstTree, game: &Gamestate, turns: (Turn, Turn)) {}
}
pub trait ExpansionPolicy {
    fn expand(&mut self, tree: &McstTree, path: &Vec<Turn>, game: &Gamestate) -> Turn;
}
pub trait DecisionPolicy {
    fn decide(&mut self, tree: &McstTree, game: &Gamestate) -> Turn;
}

pub struct McstNode {
    children: HashMap<Turn, McstNode>,
    wins: u32,
    total: u32,
    game: Gamestate,
}

impl McstNode {
    fn new(game: Gamestate) -> Self {
        McstNode {
            children: HashMap::new(),
            wins: 0,
            total: 0,
            game: game
        }
    }

    pub fn game(&self) -> &Gamestate {
        &self.game
    }

    pub fn wins(&self) -> &u32 {
        &self.wins
    }

    pub fn total(&self) -> &u32 {
        &self.total
    }

    pub fn children(&self) -> &HashMap<Turn, McstNode> {
        &self.children
    }

    fn update(&mut self, win: bool) {
        if win { self.wins += 1 };
        self.total += 1;
    }

    fn search_mut(&mut self, path: &[Turn]) -> Option<&mut McstNode> {
        if let Some(child) = &path.first() {
            if let Some(child) = self.children.get_mut(child) {
                child.search_mut(&path[1..])
            } else { None }
        } else { Some(self) }
    }

    pub fn search(&self, path: &[Turn]) -> Option<&McstNode> {
        if let Some(child) = &path.first() {
            if let Some(child) = self.children.get(child) {
                child.search(&path[1..])
            } else { None }
        } else { Some(&self) }
    }
}

pub struct McstTree {
    root: McstNode,
}

impl McstTree {
    pub fn new(game: Gamestate) -> Self {
        McstTree {
            root: McstNode::new(game),
        }
    }

    pub fn root(&self) -> &McstNode {
        &self.root
    }

    // Hmm... what we return is odd. We could return an error to distinguish
    // between running out of nodes and having an invalid path.
    // We could also use lifetimes to return an actual reference to the child
    // I think. Not sure if there's a benefit to that yet though.
    pub fn add_child(&mut self, path: &[Turn], link: Turn) -> Option<()> {
        if let Some(old) = self.root.search_mut(path) {
            if old.children.contains_key(&link) {
                None
            } else {
                let mut new_game = old.game.clone();
                if !new_game.make_move_fast(link) {
                    panic!("child didn't make real move");
                }
                let new_child = McstNode::new(new_game);
                old.children.insert(link, new_child);
                Some(())
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum CycleError {
    Selection(SelectionError),
    Expansion(ExpansionError),
    Rollout(RolloutError),
}

#[derive(Debug)]
pub enum SelectionError {
    NotANode(Vec<Turn>),  // gave us a path to a node we do not have
    NoExploration(Vec<Turn>),  // gave us a path to a node whose gamestate is terminal
}

#[derive(Debug)]
pub enum ExpansionError {
    IllegalMove(Turn),  // gave us an invalid move
    AlreadyExpanded(Turn),  // expanded to a node we already have
}

#[derive(Debug)]
pub enum RolloutError {
    IllegalMove(Vec<Turn>),
}

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
    game: Gamestate,
}

impl<
S: SelectionPolicy,
E: ExpansionPolicy,
D: DecisionPolicy,
R: Agent,
> McstAgent<S, E, D, R> {
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
            tree: McstTree::new(game.clone()),
            game: game,
        }
    }

    pub fn view_game(&self) -> &Gamestate {
        &self.game
    }

    pub fn view_tree(&self) -> &McstTree {
        &self.tree
    }

    fn select(&mut self) -> Result<Option<Vec<Turn>>, SelectionError> {
        if let Some(path) = self.selector.select(&self.tree, &self.game) {
            if let Some(node) = &self.tree.root.search(&path) {
                if node.game().get_moves().is_empty() {
                    Err(SelectionError::NoExploration(path))
                } else {
                    Ok(Some(path))
                }
            } else { Err(SelectionError::NotANode(path)) }
        } else { Ok(None) }
    }

    // path *must* refer to a valid node - will panic otherwise
    // Ok value guaranteed to return an unexpanded node
    fn expand(&mut self, path: &Vec<Turn>) -> Result<Turn, ExpansionError> {
        let link = self.expander.expand(&self.tree, path, &self.game);
        let node = self.node_from_path(path);
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

    fn rollout(&mut self, path: &Vec<Turn>, mut my_turn: bool) -> Result<bool, RolloutError> {
        let mut game = self.node_from_path(path).game().clone();
        // TODO: optimize by removing move_history?
        let mut move_history: Vec<Turn> = Vec::new();
        let my_color = match self.game.whose_turn() {
            States::Taken(c) => c,
            States::Empty => panic!("initial game is over?"),
        };

        loop {
            let valid_moves = game.get_moves();
            if !valid_moves.is_empty() {
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

    // the bool here says whether the selector decided this cycle was worth completing
    pub fn cycle(&mut self) -> Result<bool, CycleError> {
        let path = self.select();
        let mut path = match path {
            Err(e) => return Err(CycleError::Selection(e)),
            Ok(Some(path)) => path,
            Ok(Option::None) => return Ok(false),
        };

        match self.expand(&path) {
            Err(e) => return Err(CycleError::Expansion(e)),
            Ok(expansion) => {
                self.tree
                    .add_child(&path, expansion)
                    .expect("Failed to add child from expansion");
                path.push(expansion);
            },
        };

        let win = match self.rollout(&path, path.len() & 1 == 0) {
            Err(e) => return Err(CycleError::Rollout(e)),
            Ok(win) => win,
        };

        for index in 0..path.len() {
            self.node_from_path_mut(&path[..index])
                .update(win);
        }

        Ok(true)
    }

    // returns none if turn is not valid
    pub fn decide(&mut self) -> Option<Turn> {
        let decision = self.decider.decide(&self.tree, &self.game);
        if self.game.get_moves().contains(&decision) {
            Some(decision)
        } else {
            None
        }
    }

    // path must point to a valid node, will panic otherwise
    fn node_from_path_mut(&mut self, path: &[Turn]) -> &mut McstNode {
        self.tree
            .root
            .search_mut(path)
            .expect("Node from path given invalid path")
    }

    // path must point to a valid node, will panic otherwise
    fn node_from_path(&self, path: &[Turn]) -> &McstNode {
        self.tree
            .root
            .search(path)
            .expect("Node from path given invalid path")
    }

    pub fn next_two_moves(&mut self, mv1: Turn, mv2: Turn) -> bool {
        let mut test_game = self.game.clone();
        if !test_game.make_moves_fast(&[mv1, mv2]) {
            false
        } else {
            self.game.make_moves_fast(&[mv1, mv2]);
            // add first and second children if not in tree, then replace root
            if !self.tree.root.children.contains_key(&mv1) {
                if let Option::None = self.tree.add_child(&[], mv1) {
                    panic!("");
                }
            }
            if !self.tree.root.children.get(&mv1).unwrap().children.contains_key(&mv2) {
                self.tree.add_child(&[mv1], mv2);
            }
            self.tree.root = self.tree
                                 .root
                                 .children
                                 .get_mut(&mv1)
                                 .unwrap()
                                 .children
                                 .remove(&mv2)
                                 .unwrap();

            self.selector.turns_passed(&self.tree, &self.game, (mv1, mv2));
            true
        }
    }
}
