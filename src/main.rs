#![recursion_limit = "256"]

mod mechanics;
pub mod gameplay;
pub mod agent;
pub mod mcst;
pub mod data;
pub mod neural;

use std::cmp::Ordering;
use std::io::stdin;
use std::env;

use burn::backend::{Autodiff, Wgpu};
use burn::optim::AdamConfig;

use agent::{play_memory_agents, play_memory_agents_from};
use agent::implementations::{BfsExpansion, McstMemoryAgent, RandomAgent, UctDecision, UctSelection};
use gameplay::{Gamestate, Players, States};
use mcst::{benchmark, McstAgent};
use data::{collect_mcst_data, turns_to_str, BfsAllGamestates};

use neural::model_a;
use neural::model_b;

fn main() {
    let artifact_dir = &env::args().collect::<Vec<String>>()[1];

//    loop {
//        collect_mcst_data();
//    }

//    let mut uct_test = McstAgent::new(
//        UctSelection::new(2_f64.sqrt()),
//        BfsExpansion {},
//        UctDecision {},
//        RandomAgent::new(),
//        RandomAgent::new(),
//        Gamestate::new(),
//    );
//    println!("{}", benchmark(uct_test));
//    return;

    type MyBackend = Wgpu<f32, i32>;
    type MyAutodiffBackend = Autodiff<MyBackend>;

    let device = burn::backend::wgpu::WgpuDevice::default();
    model_a::train::<MyAutodiffBackend>(
        artifact_dir,
        model_a::TrainingConfig::new(model_a::ModelConfig::new(), AdamConfig::new()),
        device.clone(),
    );

    return;

    let c_time = 5;
    let _ranking: [[f64; 8]; 8] = [
        [0.64, 0.52, 0.52, 0.52, 0.54, 0.53, 0.53, 0.68],
        [0.50, 0.38, 0.47, 0.43, 0.46, 0.49, 0.35, 0.53],
        [0.52, 0.48, 0.47, 0.49, 0.52, 0.50, 0.50, 0.53],
        [0.50, 0.43, 0.47, 0.00, 0.00, 0.53, 0.46, 0.54],
        [0.52, 0.42, 0.49, 0.00, 0.00, 0.48, 0.46, 0.54],
        [0.50, 0.50, 0.49, 0.50, 0.50, 0.49, 0.49, 0.53],
        [0.50, 0.40, 0.47, 0.43, 0.44, 0.51, 0.36, 0.53],
        [0.63, 0.50, 0.52, 0.51, 0.54, 0.53, 0.52, 0.67],
    ];
    let mut uct0 = McstMemoryAgent::new(
        McstAgent::new(
            UctSelection::new(2_f64.sqrt()),
            BfsExpansion {},
            UctDecision {},
            RandomAgent::new(),
            RandomAgent::new(),
            Gamestate::new(),
        ),
        c_time
    );
    let mut uct1 = McstMemoryAgent::new(
        McstAgent::new(
            UctSelection::new(2_f64.sqrt()),
            BfsExpansion {},
            UctDecision {},
            RandomAgent::new(),
            RandomAgent::new(),
            Gamestate::new(),
        ),
        c_time
    );

    for g in BfsAllGamestates::new() {
        if g.whose_turn() == States::Taken(Players::White) {
            //println!("Skipping white turn");
            //continue;
        }
        //println!("starting position:\n{g}\n------------------\n");
        let (score, turns) = play_memory_agents_from(&mut uct0, &mut uct1, g.clone());
        let mut agd = g.clone();
        agd.make_moves_fast(&turns);
        //println!("{score}");
        //println!("{agd}");

        for i in (0..=turns.len()).step_by(2) {
            let mut copy = g.clone();
            if !copy.make_moves_fast(&turns[..i]) {
                panic!("AAAAAAAAA");
            }
            match score.partial_cmp(&0) {
                Some(Ordering::Greater) => println!("1.0,{}", copy.board().to_compact()),
                Some(Ordering::Less) => println!("0.0,{}", copy.board().to_compact()),
                Some(Ordering::Equal) => println!("0.5,{}", copy.board().to_compact()),
                _ => panic!("wtf"),
            };
        }

        for i in (1..=turns.len()).step_by(2) {
            let mut copy = g.clone();
            if !copy.make_moves_fast(&turns[..i]) {
                panic!("AAAAAAAAA");
            }
            let mut copy = copy.board().clone();
            copy.rotate_90();
            copy.flip_colors();
            match score.partial_cmp(&0) {
                Some(Ordering::Greater) => println!("0.0,{}", copy.to_compact()),
                Some(Ordering::Less) => println!("1.0,{}", copy.to_compact()),
                Some(Ordering::Equal) => println!("0.5,{}", copy.to_compact()),
                _ => panic!("wtf"),
            };
        }
    }

    loop {
        let (score, turns) = play_memory_agents(&mut uct0, &mut uct1);
        match score.partial_cmp(&0) {
            Some(Ordering::Greater) => println!("0.0:{}", turns_to_str(&turns)),
            Some(Ordering::Less) => println!("1.0:{}", turns_to_str(&turns)),
            Some(Ordering::Equal) => println!("0.5:{}", turns_to_str(&turns)),
            _ => panic!("wtf"),
        };
    }
}
