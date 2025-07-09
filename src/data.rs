use std::collections::{HashMap, VecDeque};

use magpie::othello::Game;

use crate::agent::implementations::{BfsExpansion, McstMemoryAgent, RandomAgent, UctDecision, UctSelection};
use crate::agent::{Agent, MemoryAgent};
use crate::gameplay::{str_to_loc, Gamestate, Players, States, Turn};
use crate::mcst::{McstAgent, McstNode, McstTree};
use crate::mechanics::Board;

#[derive(PartialEq)]
enum BAGState {
    Unbegun,
    InitLevel,
    ScanLevel,
    Exhausted,
}

pub struct BfsAllGamestates {
    state: Gamestate,
    board: Board,
    turns: Vec<Turn>,
    flips: Vec<Vec<(u8, u8)>>,
    level: usize,
    status: BAGState,
}

impl BfsAllGamestates {
    pub fn new() -> Self {
        BfsAllGamestates {
            state: Gamestate::new(),
            board: Board::new(),
            turns: Vec::new(),
            flips: Vec::new(),
            level: 0,
            status: BAGState::Unbegun,
        }
    }

    // invariants: state is current state, turns is all turns to state,
    // flips is all flips to state, level is target level > turns.len()
    fn go_down_from_down(&mut self) {
        let turns = self.state.get_moves();
        if turns.is_empty() {
            self.go_back();
        } else {
            self.turns.push(turns[0]);
            let f = self.state.make_move(turns[0]).unwrap();
            self.flips.push(f);
            if self.turns.len() == self.level {
                assert!(self.status == BAGState::InitLevel || self.status == BAGState::ScanLevel);
                self.status = BAGState::ScanLevel;
            } else {
                self.go_down_from_down();
            }
        }
    }

    // invariants: state is current state, turns is all turns to state,
    // flips is all flips to state, level is target level > turns.len(),
    // from is the turn we have just exited from
    fn go_down_from_back(&mut self, from: Turn) {
        let turns = self.state.get_moves();
        if turns.is_empty() {
            self.go_back();
        } else {
            let i = turns.iter().position(|t| *t == from).unwrap();
            if i == turns.len() - 1 {
                self.go_back();
            } else {
                self.turns.push(turns[i + 1]);
                let f = self.state.make_move(turns[i + 1]).unwrap();
                self.flips.push(f);
                if self.turns.len() == self.level {
                    assert!(self.status == BAGState::InitLevel || self.status == BAGState::ScanLevel);
                    self.status = BAGState::ScanLevel;
                } else {
                    self.go_down_from_down();
                }
            }
        }
    }

    // Goes backwards, handling updating the turns and flips vecs.
    fn go_back(&mut self) {
        if let Some(turn) = self.turns.pop() {
            // undo a turn - unflip pieces and remove placed piece if not pass
            let flipped_color = if self.turns.len() % 2 == 0 { Players::White } else { Players::Black };
            self.board = self.state.board().clone();

            for (x, y) in self.flips.pop().unwrap() {
                self.board.change(x, y, States::Taken(flipped_color));
            }
            if let Some((x, y)) = turn {
                self.board.change(x, y, States::Empty);
            }
            self.state = Gamestate::new_from(self.board, u8::try_from(self.turns.len()).unwrap());
            assert!(self.state.get_moves().contains(&turn));
            self.go_down_from_back(turn);
        } else {
            // we back at the start
            match self.status {
                BAGState::InitLevel => {
                    // searched for a level and couldn't find it
                    self.status = BAGState::Exhausted;
                }
                BAGState::ScanLevel => {
                    // finished scanning current level, go one down
                    self.level += 1;
                    self.status = BAGState::InitLevel;
                    self.go_down_from_down();
                },
                BAGState::Unbegun | BAGState::Exhausted => panic!("This method shouldn't have been called."),
            };
        }
    }
}

impl Iterator for BfsAllGamestates {
    type Item = Gamestate;

    fn next(&mut self) -> Option<Self::Item> {
        match self.status {
            BAGState::Unbegun => {
                self.status = BAGState::ScanLevel;
                Some(self.state.clone())
            },
            BAGState::InitLevel => {
                panic!("This state should not be possible to be in.");
            },
            BAGState::ScanLevel => {
                // TODO: Should this use trampolining?
                self.go_back();
                if self.status == BAGState::Exhausted {
                    None
                } else {
                    Some(self.state.clone())
                }
            },
            BAGState::Exhausted => {
                None
            },
        }
    }
}

/// Converts a list of turns to a String representing them.
pub fn turns_to_str(turns: &[Turn]) -> String {
    turns.iter().map(
        |t: &Turn| -> String {
            if let Some((x, y)) = t {
                format!("{x},{y}")
            } else {
                String::from("")
            }
        }
    ).collect::<Vec<String>>().join(";")
}

pub fn str_to_turns(string: &str) -> Option<Vec<Turn>> {
    let mut turns: Vec<Turn> = Vec::new();
    for trial in string.split(";") {
        if trial == "" {
            turns.push(None);
        } else {
            if let Some(loc) = str_to_loc(trial) {
                turns.push(Some(loc))
            } else {
                return None;
            }
        }
    }
    Some(turns)
}

pub fn turns_to_game_seeded(turns: &[Turn], mut g: Gamestate) -> Option<Vec<Gamestate>> {
    let mut v = vec![g.clone()];

    for t in turns {
        if g.make_move_fast(*t) {
            v.push(g.clone());
        } else {
            return None;
        }
    }

    Some(v)
}

pub fn turns_to_game(turns: &[Turn]) -> Option<Vec<Gamestate>> {
    turns_to_game_seeded(turns, Gamestate::new())
}

pub fn str_to_states(line: &str) -> (f32, Vec<Board>, Vec<Board>) {
    let record: Vec<&str> = line.split(":").collect();
    let score: f32 = record[0].parse().unwrap();
    // you will probably have to do better error handling here one day
    let games = turns_to_game(&str_to_turns(record[1]).unwrap()).unwrap();
    let mut boards: Vec<Board> = Vec::new();
    let mut rot_boards: Vec<Board> = Vec::new();

    // Generate rotated versions of the game
    for (index, game) in games.iter().enumerate() {
        if index % 2 == 0 {
            boards.push(game.board().clone());
        } else {
            let mut rot = game.board().clone();
            rot.rotate_90();
            rot.flip_colors();
            rot_boards.push(rot);
        }
    };

    (score, boards, rot_boards)
}

pub fn game_states_records(contents: &str) -> HashMap<u128, f32> {
    let mut all_games = HashMap::<u128, (f32, f32)>::new();
    for line in contents.split("\n") {
        if line == "" {
            continue;
        }
        let (score, first, second) = str_to_states(line);
        for game in &first {
            let entry = all_games.entry(game.to_compact()).or_insert((0.0, 0.0));
            entry.0 += 1.0 - score;
            entry.1 += 1.0; // total
        }
        for game in &second {
            let entry = all_games.entry(game.to_compact()).or_insert((0.0, 0.0));
            entry.0 += score;
            entry.1 += 1.0; // total
        }
    }

    all_games.into_iter()
        .map(|(k, (numerator, denominator))| (k, numerator / denominator))
        .collect()
}

pub fn collect_mcst_data() {
    let mut g = Gamestate::new();
    let r = RandomAgent::new();

    while !g.get_moves().is_empty() {
        let mut a = McstAgent::new(
            UctSelection::new(2_f64.sqrt()),
            BfsExpansion {},
            UctDecision {},
            RandomAgent::new(),
            RandomAgent::new(),
            g.clone(),
        );
        for _ in 0..100000 {
            let _ = a.cycle();
        }

        let mut data = HashMap::<u128, (u64, u64)>::new();
        mcst_node_report(a.tree().root(), &mut data);
        for (compact, (win, total)) in data.iter() {
            println!("{},{},{}", compact, win, total);
        }

        g.make_move_fast(r.make_move(&g));
        if !g.get_moves().is_empty() {
            g.make_move_fast(r.make_move(&g));
        }
    }
}

pub fn mcst_node_report(node: &McstNode, data: &mut HashMap<u128, (u64, u64)>) {
    if node.total() >= &64 {
        let entry = data.entry(node.game().board().to_compact()).or_insert((0, 0));
        entry.0 += u64::from(*node.wins());
        entry.1 += u64::from(*node.total());
        for child in node.children().values() {
            mcst_node_skip(child, data);
        }
    }
}

pub fn mcst_node_skip(node: &McstNode, data: &mut HashMap<u128, (u64, u64)>) {
    if node.total() >= &64 {
        for child in node.children().values() {
            mcst_node_report(child, data);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bfsallgamestates() {
        let mut q = VecDeque::<Gamestate>::new();
        q.push_back(Gamestate::new());

        for g in BfsAllGamestates::new().take(10000) {
            let expected = q.pop_front().unwrap();
            for t in expected.get_moves().iter() {
                let mut child = expected.clone();
                child.make_move_fast(*t);
                q.push_back(child);
            }
            assert_eq!(g.board(), expected.board());
        }
    }

    #[test]
    fn test_turns_to_str() {
        assert_eq!(turns_to_str(&[Some((1, 2)), Some((3, 4)), None]), "1,2;3,4;");
    }

    #[test]
    fn test_str_to_turns() {
        assert_eq!(str_to_turns("1,2;3,4;"), Some(vec![Some((1, 2)), Some((3, 4)), None]));
    }

    #[test]
    fn test_turns_to_game() {
        let mut g = Gamestate::new();
        let mut v = vec![g.clone()];
        g.make_move_fast(Some((4, 5)));
        v.push(g.clone());
        g.make_move_fast(Some((3, 5)));
        v.push(g.clone());
        assert_eq!(turns_to_game(&[Some((4_u8, 5_u8)), Some((3_u8, 5_u8))]), Some(v));
    }

    #[test]
    fn test_str_to_states() {
        let (score, first, second) = str_to_states("1.0:4,5;5,3;3,2;2,3");

        let moves = [Some((4, 5)), Some((5, 3)), Some((3, 2)), Some((2, 3))];
        let mut g = Gamestate::new();
        let mut b: Board;
        let mut first_ex = Vec::<Board>::new();
        let mut second_ex = Vec::<Board>::new();

        first_ex.push(g.board().clone());
        g.make_move_fast(moves[0]);
        b = g.board().clone();
        b.rotate_90();
        b.flip_colors();
        second_ex.push(b);
        g.make_move_fast(moves[1]);
        first_ex.push(g.board().clone());
        g.make_move_fast(moves[2]);
        b = g.board().clone();
        b.rotate_90();
        b.flip_colors();
        second_ex.push(b);
        g.make_move_fast(moves[3]);
        first_ex.push(g.board().clone());

        assert_eq!(score, 1.0);
        assert_eq!(first, first_ex);
        assert_eq!(second, second_ex);
    }

    #[test]
    fn test_game_states_record() {
        let records = game_states_records("0.0:4,5;5,3;3,2;2,3\n1.0:4,5;5,5\n");

        let mut expected = HashMap::<u128, f32>::new();
        let mut g = Gamestate::new();
        let mut g2: Gamestate;
        let mut b: Board;

        expected.insert(g.board().to_compact(), 0.5); // initial state (350258943680422884)

        g.make_move_fast(Some((4, 5)));
        b = g.board().clone();
        b.rotate_90();
        b.flip_colors();
        expected.insert(b.to_compact(), 0.5); // 4,5 (650448214274421126)
        g2 = g.clone();

        g.make_move_fast(Some((5, 3)));
        expected.insert(g.board().to_compact(), 1.0); // 4,5;5,3 (657214414548447576087)

        g2.make_move_fast(Some((5,5)));
        expected.insert(g2.board().to_compact(), 0.0); // 4,5;5,5 (5909425955951238817533)

        g.make_move_fast(Some((3, 2)));
        b = g.board().clone();
        b.rotate_90();
        b.flip_colors();
        expected.insert(b.to_compact(), 0.0); // 4,5;5,5,3;3,2 (657214409464715919429)

        g.make_move_fast(Some((2, 3)));
        expected.insert(g.board().to_compact(), 1.0); // 4,5;5,3;3,2;2,3 (657214417092637927350)

        assert_eq!(
            records,
            expected
        );
    }
}
