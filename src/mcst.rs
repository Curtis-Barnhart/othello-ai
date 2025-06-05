use std::collections::HashMap;
use std::cmp::Ordering;

use crate::agent::Agent; use crate::gameplay::{Players, Gamestate};

pub trait SelectionPolicy {
    fn select(&mut self, tree: &McstTree, game: &Gamestate) -> Option<Vec<(u8, u8)>>;
    fn turns_passed(&mut self, tree: &McstTree, game: &Gamestate, turns: ((u8, u8), (u8, u8))) {}
}
pub trait ExpansionPolicy {
    fn expand(&mut self, tree: &McstTree, path: &Vec<(u8, u8)>, game: &Gamestate) -> (u8, u8);
}
pub trait DecisionPolicy {
    fn decide(&mut self, tree: &McstTree, game: &Gamestate) -> (u8, u8);
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

    fn search_mut(&mut self, path: &[(u8, u8)]) -> Option<&mut McstNode> {
        if let Some(child) = &path.first() {
            if let Some(child) = self.children.get_mut(child) {
                child.search_mut(&path[1..])
            } else { None }
        } else { Some(self) }
    }

    pub fn search(&self, path: &[(u8, u8)]) -> Option<&McstNode> {
        if let Some(child) = &path.first() {
            if let Some(child) = self.children.get(child) {
                child.search(&path[1..])
            } else { None }
        } else { Some(&self) }
    }
}

#[derive(Debug)]
pub struct McstTree {
    pub root: McstNode,
//    pub max_nodes: u32,
//    pub used_nodes: u32,
}

impl McstTree {
    pub fn new(max_nodes: u32) -> Self {
        McstTree {
            root: McstNode::new(),
        }
    }

    // Hmm... what we return is odd. We could return an error to distinguish
    // between running out of nodes and having an invalid path.
    // We could also use lifetimes to return an actual reference to the child
    // I think. Not sure if there's a benefit to that yet though.
    pub fn add_child(&mut self, path: &[(u8, u8)], link: (u8, u8)) -> Option<()> {
        if let Some(old) = self.root.search_mut(path) {
            if old.children.contains_key(&link) {
                None
            } else {
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
    D: DecisionPolicy,
    R: Agent,
> {
    selector: S,
    expander: E,
    rollout: R,
    opponent: R,
    decider: D,
    tree: McstTree,
    // TODO: fix this so there's a way to set the first game
    pub game: crate::gameplay::Gamestate,
}

#[derive(Debug)]
pub enum CycleError {
    Selection(SelectionError),
    Expansion(ExpansionError),
    Rollout(RolloutError),
}

#[derive(Debug)]
pub enum SelectionError {
    NotANode(Vec<(u8, u8)>),  // gave us a path to a node we do not have
    NoExploration(Vec<(u8, u8)>),  // gave us a path to a node whose gamestate is terminal
}

#[derive(Debug)]
pub enum ExpansionError {
    IllegalMove((u8, u8)),  // gave us an invalid move
    AlreadyExpanded((u8, u8)),  // expanded to a node we already have
}

#[derive(Debug)]
pub enum RolloutError {
    IllegalMove(Vec<(u8, u8)>),
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
        max_nodes: u32,
    ) -> Self {
        McstAgent {
            selector: selector,
            expander: expander,
            decider: decider,
            rollout: rollout,
            opponent: opponent,
            tree: McstTree::new(max_nodes),
            game: crate::gameplay::Gamestate::new()
        }
    }

    pub fn view_game(&self) -> &Gamestate {
        &self.game
    }

    pub fn view_tree(&self) -> &McstTree {
        &self.tree
    }

    // Ok value is guaranteed to be a node
    fn select(&mut self) -> Result<Option<Vec<(u8, u8)>>, SelectionError> {
        if let Some(path) = self.selector.select(&self.tree, &self.game) {
            if let Some(_) = &self.tree.root.search(&path) {
                let selected_game = self.game_from_path(&path);
                if selected_game.get_moves().is_empty() {
                    Err(SelectionError::NoExploration(path))
                } else {
                    Ok(Some(path))
                }
            } else { Err(SelectionError::NotANode(path)) }
        } else { Ok(None) }
    }

    // path *must* refer to a valid node - will panic otherwise
    // Ok value guaranteed to return an unexpanded node
    fn expand(&mut self, path: &Vec<(u8, u8)>) -> Result<Option<(u8, u8)>, ExpansionError> {
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

    fn rollout(&mut self, path: &Vec<(u8, u8)>, mut my_turn: bool) -> Result<bool, RolloutError> {
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

        for index in 0..path.len() {
            self.node_from_path_mut(&path[..index])
                .update(win);
        }

        Ok(true)
    }

    pub fn decide(&mut self) -> Option<(u8, u8)> {
        let decision = self.decider.decide(&self.tree, &self.game);
        if self.game.get_moves().contains(&decision) {
            Some(decision)
        } else {
            None
        }
    }

    // path must point to a valid node, will panic otherwise
    fn node_from_path_mut(&mut self, path: &[(u8, u8)]) -> &mut McstNode {
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

    pub fn next_two_moves(&mut self, mv1: (u8, u8), mv2: (u8, u8)) -> bool {
        let mut test_game = self.game.clone();
        if !test_game.make_turns(&[mv1, mv2]) {
            false
        } else {
            self.game.make_turns(&[mv1, mv2]);
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
