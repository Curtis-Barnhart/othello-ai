pub mod mechanics {
    use std::fmt;

    static AROUND: [(u8, u8); 8] = [
        (255, 1),   (0, 1),   (1, 1),
        (255, 0),             (1, 0),
        (255, 255), (0, 255), (1, 255),
    ];

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Players {
        White,
        Black,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum States {
        Taken(Players),
        Empty,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Board {
        pieces: [[States; 8]; 8],
    }

    impl fmt::Display for Board {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                " 01234567\n{}",
                self.pieces.iter().enumerate().map(
                    |(index, inner)| -> String {
                        index.to_string() + &inner.iter().map(
                            |tile: &States| -> String {
                                match tile {
                                    States::Empty => String::from("."),
                                    States::Taken(Players::Black) => String::from("B"),
                                    States::Taken(Players::White) => String::from("W"),
                                }
                            }
                        ).collect::<Vec<String>>().join("")
                    }
                ).collect::<Vec<String>>().join("\n")
            )
        }
    }

    #[derive(Debug, PartialEq)]
    enum FlipType {
        Valid,
        Degenerate,
        Invalid,
    }

    impl Board {
        pub fn new() -> Self {
            Board {
                pieces: [[States::Empty; 8]; 8],
            }
        }

        pub fn score(&self) -> i8 {
            self.pieces.iter().map(
                |row: &[States; 8]| -> i8 {
                    row.iter().map(
                        |piece: &States| -> i8 {
                            match piece {
                                States::Empty => 0,
                                States::Taken(Players::Black) => 1,
                                States::Taken(Players::White) => -1
                            }
                        }
                    ).sum()
                }
            ).sum()
        }

        // does not check x and y
        pub fn change(&mut self, x: u8, y: u8, val: States) {
            self.pieces[usize::from(y)][usize::from(x)] = val;
        }

        // Returns None if location is off the board
        pub fn at(&self, x: u8, y: u8) -> Option<States> {
            if x < 8 && y < 8 {
                return Some(self.pieces[usize::from(y)][usize::from(x)]);
            }
            None
        }

        // does not check x and y values for being on board
        // If it goes off the side it returns None
        // If it has no opposite color in between it returns Some(false)
        fn can_flip_toward_help(&self, x: u8, y: u8, dx: u8, dy: u8, origin: Players) -> FlipType {
            let new_x = x.wrapping_add(dx);
            let new_y = y.wrapping_add(dy);
            if let Some(States::Taken(new_player)) = self.at(new_x, new_y) {
                if origin != new_player {
                    if self.can_flip_toward_help(new_x, new_y, dx, dy, origin) != FlipType::Invalid {
                        FlipType::Valid
                    } else { FlipType::Invalid }
                } else { FlipType::Degenerate }
            } else { FlipType::Invalid }
        }

        // does not check x and y values for being on board
        fn can_flip_toward(&self, x: u8, y: u8, dx: u8, dy: u8, origin: Players) -> bool {
            self.can_flip_toward_help(x, y, dx, dy, origin) == FlipType::Valid
        }

        // handles all values of x and y
        pub fn can_move(&self, x: u8, y: u8, p: Players) -> bool {
            if let Some(States::Empty) = self.at(x, y) {
                for (dx, dy) in AROUND {
                    if self.can_flip_toward(x, y, dx, dy, p) {
                        return true;
                    }
                }
            }
            false
        }

        pub fn get_moves(&self, p: Players) -> Vec<(u8, u8)> {
            let mut v: Vec<(u8, u8)> = Vec::new();
            v.reserve(64);
            for x in 0..8 {
                for y in 0..8 {
                    if self.can_move(x, y, p) {
                        v.push((x, y));
                    }
                }
            }
            v.shrink_to_fit();
            v
        }

        // does not check x and y values for being on board
        // If it goes off the side it returns None
        // If it has no opposite color in between it returns an empty vec
        fn flip_toward(&mut self, x: u8, y: u8, dx: u8, dy: u8, origin: Players) -> Option<Vec<(u8, u8)>> {
            let new_x = x.wrapping_add(dx);
            let new_y = y.wrapping_add(dy);
            if let Some(States::Taken(new_player)) = self.at(new_x, new_y) {
                if origin != new_player {
                    if let Some(mut future_list) = self.flip_toward(new_x, new_y, dx, dy, origin) {
                        self.change(new_x, new_y, States::Taken(origin));
                        future_list.push((new_x, new_y));
                        Some(future_list)
                    } else { None }
                } else { Some(Vec::new()) }
            } else {
                None
            }
        }

        // handles any values of x and y
        // assumes the origin move has been made
        pub fn flip_all(&mut self, x: u8, y: u8) -> Vec<(u8, u8)> {
            let mut places: Vec<(u8, u8)> = Vec::new();
            if let Some(States::Taken(origin)) = self.at(x, y) {
                for (dx, dy) in AROUND {
                    if let Some(partial) = self.flip_toward(x, y, dx, dy, origin) {
                        places.extend(partial);
                    }
                }
            }
            places
        }
    }
}

pub mod gameplay {
    use std::fmt;

    pub use crate::mechanics::Players;
    pub use crate::mechanics::States;

    #[derive(Clone)]
    pub struct Gamestate {
        board: crate::mechanics::Board,
        history: Vec<(u8, u8)>,
    }

    impl fmt::Display for Gamestate {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "{}\n{} to move",
                self.board,
                if self.whose_turn() == Players::Black { "Black" } else { "White" },
            )
        }
    }

    impl Gamestate {
        pub fn new() -> crate::gameplay::Gamestate {
            let mut g = Gamestate {
                board: crate::mechanics::Board::new(),
                history: Vec::new(),
            };
            g.board.change(3, 3, States::Taken(Players::White));
            g.board.change(4, 4, States::Taken(Players::White));
            g.board.change(4, 3, States::Taken(Players::Black));
            g.board.change(3, 4, States::Taken(Players::Black));
            g
        }

        pub fn whose_turn(&self) -> Players {
            if self.history.len() & 1 == 0 { Players::Black } else { Players::White }
        }

        pub fn score(&self) -> i8 {
            self.board.score()
        }

        pub fn get_moves(&self) -> Vec<(u8, u8)> {
            self.board.get_moves(self.whose_turn())
        }

        pub fn view_board(&self) -> &crate::mechanics::Board {
            &self.board
        }

        pub fn view_history(&self) -> &Vec<(u8, u8)> {
            &self.history
        }

        pub fn make_turn(&mut self, x: u8, y: u8) -> Vec<(u8, u8)> {
            if let Some(States::Empty) = self.board.at(x, y) {
                self.board.change(x, y, States::Taken(self.whose_turn()));
                let v = self.board.flip_all(x, y);
                if v.is_empty() {
                    self.board.change(x, y, States::Empty);
                } else {
                    self.history.push((x, y));
                }
                v
            } else {
                Vec::new()
            }
        }
    }

    pub fn str_to_loc(s: &str) -> Option<(u8, u8)> {
        let stripped = s.replace(" ", "");
        let mut iter = stripped.split(",");
        if let (Some(x), Some(y)) = (iter.next(), iter.next()) {
            if let (Ok(x), Ok(y)) = (x.parse::<u8>(), y.parse::<u8>()) {
                Some((x, y))
            } else { None }
        } else { None }
    }
}

pub mod agent {
    use std::cmp::Ordering;
    use std::io;
    use rand::prelude::IndexedRandom;
    use crate::gameplay;

    pub trait Agent {
        fn make_move(&self, state: &gameplay::Gamestate) -> (u8, u8);
    }

    pub struct RandomAgent {}

    impl Agent for RandomAgent {
        fn make_move(&self, state: &gameplay::Gamestate) -> (u8, u8) {
            let mut rng = rand::rng();
            state.get_moves()
                 .choose(&mut rng)
                 .copied()
                 .expect("There were no valid moves.")
        }
    }

    pub struct GreedyAgent {}

    impl Agent for GreedyAgent {
        fn make_move(&self, state: &gameplay::Gamestate) -> (u8, u8) {
            state.get_moves()
                 .iter()
                 .max_by(|(x1, y1), (x2, y2)| -> Ordering {
                     // TODO: figure out wth derefing does to borrowing
                     let v1 = state.clone().make_turn(*x1, *y1).len();
                     let v2 = state.clone().make_turn(*x2, *y2).len();
                     v1.cmp(&v2)
                 })
                 .copied()
                 .expect("There were no valid moves.")

        }
    }

    pub struct HumanAgent {}

    impl Agent for HumanAgent {
        fn make_move(&self, state: &gameplay::Gamestate) -> (u8, u8) {
            let stdin = io::stdin();
            let mut input = String::new();
            let valid_moves = state.get_moves();

            loop {
                println!("Enter a coordinate:");
                input.clear();
                stdin.read_line(&mut input).expect("stdio could not be read from");
                input.pop();

                if let Some((x, y)) = crate::gameplay::str_to_loc(&input) {
                    if valid_moves.contains(&(x, y)) {
                        break (x, y)
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

    pub struct HumanDebugger {}

    impl Agent for HumanDebugger {
        fn make_move(&self, state: &gameplay::Gamestate) -> (u8, u8) {
            let stdin = io::stdin();
            let mut input = String::new();
            let valid_moves = state.get_moves();

            loop {
                println!("Enter a coordinate:");
                input.clear();
                stdin.read_line(&mut input).expect("stdio could not be read from");
                input.pop();

                if input == "/moves" {
                    println!("{}", valid_moves.iter().map(
                        |(x, y)| -> String { format!("({}, {})", x, y) }
                    ).collect::<Vec<String>>().join(", "));
                } else if input == "/history" {
                    println!("{}", state.view_history().iter().map(
                        |(x, y)| -> String { format!("({}, {})", x, y) }
                    ).collect::<Vec<String>>().join(", "));
                } else {
                    if let Some((x, y)) = crate::gameplay::str_to_loc(&input) {
                        if valid_moves.contains(&(x, y)) {
                            break (x, y)
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

pub mod mcst {
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
}

use crate::agent::Agent;
use crate::mcst::{SelectionPolicy, ExpansionPolicy, DecisionPolicy};

struct BasicSelection {}

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
