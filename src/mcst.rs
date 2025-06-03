use std::collections::HashMap;
use std::cmp::Ordering;

use crate::agent::Agent;
use crate::gameplay::{Players, Gamestate};

pub trait SelectionPolicy {
    fn select(&self, tree: &McstTree, game: &Gamestate) -> Vec<(u8, u8)>;
}
pub trait ExpansionPolicy {
    fn expand(&self, tree: &McstTree, path: &Vec<(u8, u8)>, game: &Gamestate) -> (u8, u8);
}
pub trait DecisionPolicy {
    fn decide(&self, tree: &McstTree, game: &Gamestate) -> (u8, u8);
}

#[derive(Debug)]
pub struct McstNode {
    pub children: HashMap<(u8, u8), McstNode>,
    pub wins: u32,
    pub total: u32,
}

impl McstNode {
    fn new() -> Self {
        McstNode {
            children: HashMap::new(),
            wins: 0,
            total: 0,
        }
    }

    fn update(&mut self, win: bool) {
        if win { self.wins += 1 };
        self.total += 1;
    }

    fn search_mut(&self, path: &[(u8, u8)]) -> Option<&mut McstNode> {
        if let Some(child) = &path.first() {
            if let Some(child) = self.children.get(child) {
                child.search_mut(&path[1..])
            } else { None }
        } else { None }
    }

    fn search(&self, path: &[(u8, u8)]) -> Option<&McstNode> {
        if let Some(child) = &path.first() {
            if let Some(child) = self.children.get(child) {
                child.search(&path[1..])
            } else { None }
        } else { None }
    }
}

#[derive(Debug)]
pub struct McstTree {
    pub root: McstNode,
    pub max_nodes: u32,
    pub used_nodes: u32,
}

impl McstTree {
    pub fn new(max_nodes: u32) -> Self {
        McstTree {
            root: McstNode::new(),
            max_nodes: max_nodes,
            used_nodes: 0,
        }
    }

    pub fn node_usage(&self) -> u32 {
        self.max_nodes - self.used_nodes
    }

    // Hmm... what we return is odd. We could return an error to distinguish
    // between running out of nodes and having an invalid path.
    // We could also use lifetimes to return an actual reference to the child
    // I think. Not sure if there's a benefit to that yet though.
    pub fn add_child(&mut self, path: &[(u8, u8)], link: (u8, u8)) -> Option<()> {
        if let Some(old) = self.root.search_mut(path) {
            if old.children.contains_key(&link) || (self.used_nodes == self.max_nodes) {
                None
            } else {
                self.used_nodes += 1;
                let new_child = McstNode::new();
                old.children.insert(link, new_child);
                Some(())
            }
        } else {
            None
        }
    }
}

pub struct McstAgent<
S: SelectionPolicy,
E: ExpansionPolicy,
R: Agent,
> {
    selector: S,
    expander: E,
    rollout: R,
    opponent: R,
    tree: McstTree,
    game: crate::gameplay::Gamestate,
}

pub enum CycleError {
    Selection(SelectionError),
    Expansion(ExpansionError),
    Rollout(RolloutError),
}

pub enum SelectionError {
    NotANode(Vec<(u8, u8)>),  // gave us a path to a node we do not have
    NoExploration(Vec<(u8, u8)>),  // gave us a path to a node whose gamestate is terminal
}

pub enum ExpansionError {
    IllegalMove((u8, u8)),  // gave us an invalid move
    AlreadyExpanded((u8, u8)),  // expanded to a node we already have
}

pub enum RolloutError {
    IllegalMove(Vec<(u8, u8)>),
}

impl<
S: SelectionPolicy,
E: ExpansionPolicy,
R: Agent,
> McstAgent<S, E, R> {
    pub fn new(
        selector: S,
        expander: E,
        rollout: R,
        opponent: R,
        max_nodes: u32,
    ) -> Self {
        McstAgent {
            selector: selector,
            expander: expander,
            rollout: rollout,
            opponent: opponent,
            tree: McstTree::new(max_nodes),
            game: crate::gameplay::Gamestate::new()
        }
    }

    fn view_game(&self) -> &Gamestate {
        &self.game
    }

    // Ok value is guaranteed to be a node
    fn select(&self) -> Result<Vec<(u8, u8)>, SelectionError> {
        let path = self.selector.select(&self.tree, &self.game);
        if let Some(_) = &self.tree.root.search(&path) {
            let selected_game = self.game_from_path(&path);
            if selected_game.get_moves().is_empty() {
                Err(SelectionError::NoExploration(path))
            } else {
                Ok(path)
            }
        } else { Err(SelectionError::NotANode(path)) }
    }

    // path *must* refer to a valid node - will panic otherwise
    // Ok value guaranteed to return an unexpanded node
    fn expand(&self, path: &Vec<(u8, u8)>) -> Result<Option<(u8, u8)>, ExpansionError> {
        let game = self.game_from_path(path);
        let link = self.expander.expand(&self.tree, path, &self.game);
        if game.get_moves().contains(&link) {
            if self.node_from_path(&path)
                .children
                    .contains_key(&link) {
                        Err(ExpansionError::AlreadyExpanded(link))
                    } else {
                        Ok(Some(link))
            }
        } else {
            Err(ExpansionError::IllegalMove(link))
        }
    }

    fn rollout(&self, path: &Vec<(u8, u8)>, mut my_turn: bool) -> Result<bool, RolloutError> {
        let mut game = self.game_from_path(path);
        let mut move_history: Vec<(u8, u8)> = Vec::new();
        let my_color = if my_turn {
            game.whose_turn()
        } else {
            if game.whose_turn() == Players::Black { Players::White } else { Players::Black }
        };

        loop {
            let valid_moves = game.get_moves();
            if valid_moves.is_empty() {
                break Ok(match (my_color, game.score().cmp(&0)) {
                    (Players::Black, Ordering::Greater) => true,
                    (Players::White, Ordering::Less) => true,
                    _ => false,
                });
            }

            let player_move = if my_turn {
                self.rollout.make_move(&game)
            } else {
                self.opponent.make_move(&game)
            };
            move_history.push(player_move);

            game.make_turn(player_move.0, player_move.1);
            if !valid_moves.contains(&player_move) {
                break Err(RolloutError::IllegalMove(move_history));
            }
            my_turn = !my_turn;
        }
    }

    fn cycle(&mut self) -> Result<(), CycleError> {
        let path = self.select();
        let mut path = match path {
            Err(e) => return Err(CycleError::Selection(e)),
            Ok(path) => path,
        };

        match self.expand(&path) {
            Err(e) => return Err(CycleError::Expansion(e)),
            Ok(Some(expansion)) => {
                self.tree
                    .add_child(&path, expansion)
                    .expect("Failed to add child from expansion");
                path.push(expansion);
            },
            _ => (),
        };

        let win = match self.rollout(&path, path.len() & 1 == 0) {
            Err(e) => return Err(CycleError::Rollout(e)),
            Ok(win) => win,
        };

        self.node_from_path_mut(&path)
            .update(win);

        Ok(())
    }

    // path must point to a valid node, will panic otherwise
    fn node_from_path_mut(&self, path: &Vec<(u8, u8)>) -> &mut McstNode {
        self.tree
            .root
            .search_mut(&path)
            .expect("Node from path given invalid path")
    }

    // path must point to a valid node, will panic otherwise
    fn node_from_path(&self, path: &Vec<(u8, u8)>) -> &McstNode {
        self.tree
            .root
            .search(&path)
            .expect("Node from path given invalid path")
    }

    // this can only be called when path consists of valid moves
    fn game_from_path(&self, path: &Vec<(u8, u8)>) -> crate::gameplay::Gamestate {
        let mut demo = self.game.clone();
        for (x, y) in path {
            let flips = demo.make_turn(*x, *y);
            if flips.is_empty() {
                panic!("Path was invalid");
            }
        }
        demo
    }
}
