mod mechanics;
pub mod gameplay;
pub mod agent;
pub mod mcst;

use std::cmp::Ordering;

use agent::{play_memory_agents, Agent, MemorifiedAgent, MemoryAgent};
use agent::implementations::{BfsExpansion, BfsSelectionFast, McstMemoryAgent, RandomAgent, RankedCellAgent, UctDecision, UctSelection, WinAverageDecision};
use gameplay::Gamestate;
use mcst::{DecisionPolicy, McstAgent};

fn shenanigans() {
    let ranking: [[f64; 8]; 8] = [
        [0.64, 0.52, 0.52, 0.52, 0.54, 0.53, 0.53, 0.68],
        [0.50, 0.38, 0.47, 0.43, 0.46, 0.49, 0.35, 0.53],
        [0.52, 0.48, 0.47, 0.49, 0.52, 0.50, 0.50, 0.53],
        [0.50, 0.43, 0.47, 0.00, 0.00, 0.53, 0.46, 0.54],
        [0.52, 0.42, 0.49, 0.00, 0.00, 0.48, 0.46, 0.54],
        [0.50, 0.50, 0.49, 0.50, 0.50, 0.49, 0.49, 0.53],
        [0.50, 0.40, 0.47, 0.43, 0.44, 0.51, 0.36, 0.53],
        [0.63, 0.50, 0.52, 0.51, 0.54, 0.53, 0.52, 0.67],
    ];
    let mut r1 = MemorifiedAgent::<RandomAgent>::new(RandomAgent::new());
    let mut r2 = MemorifiedAgent::<RandomAgent>::new(RandomAgent::new());
    let mut bfs10 = McstMemoryAgent::new(
        McstAgent::new(
            BfsSelectionFast::new(),
            BfsExpansion {},
            WinAverageDecision {},
            RandomAgent::new(),
            RandomAgent::new(),
            Gamestate::new(),
        ),
        1,
    );
    let mut rank = MemorifiedAgent::<RankedCellAgent>::new(RankedCellAgent::new(ranking));

//    println!("random vs random (10,000): {}", crate::agent::benchmark_memory_agents(&mut r1, &mut r2, 10000));
//    println!("random vs ranked (10,000): {}", crate::agent::benchmark_memory_agents(&mut r1, &mut rank, 10000));
//    println!("random vs bfs(1) (20):    {}", crate::agent::benchmark_memory_agents(&mut r1, &mut bfs1, 20));

    println!("ranked vs bfs(1) (10):    {}", crate::agent::benchmark_memory_agents(&mut rank, &mut bfs10, 25));
}

fn main() {
    //let _ = stdin().read_line(&mut String::new());
    // from sqrt(2)/2 to 2sqrt(2)

    let c_time = 100;
    let ranking: [[f64; 8]; 8] = [
        [0.64, 0.52, 0.52, 0.52, 0.54, 0.53, 0.53, 0.68],
        [0.50, 0.38, 0.47, 0.43, 0.46, 0.49, 0.35, 0.53],
        [0.52, 0.48, 0.47, 0.49, 0.52, 0.50, 0.50, 0.53],
        [0.50, 0.43, 0.47, 0.00, 0.00, 0.53, 0.46, 0.54],
        [0.52, 0.42, 0.49, 0.00, 0.00, 0.48, 0.46, 0.54],
        [0.50, 0.50, 0.49, 0.50, 0.50, 0.49, 0.49, 0.53],
        [0.50, 0.40, 0.47, 0.43, 0.44, 0.51, 0.36, 0.53],
        [0.63, 0.50, 0.52, 0.51, 0.54, 0.53, 0.52, 0.67],
    ];
    let mut bfs100 = McstMemoryAgent::new(
        McstAgent::new(
            BfsSelectionFast::new(),
            BfsExpansion {},
            WinAverageDecision {},
            RankedCellAgent::new(ranking),
            RankedCellAgent::new(ranking),
            Gamestate::new(),
        ),
        c_time,
    );
    let mut uct100 = McstMemoryAgent::new(
        McstAgent::new(
            UctSelection::new(2_f64.sqrt()),
            BfsExpansion {},
            UctDecision {},
            RankedCellAgent::new(ranking),
            RankedCellAgent::new(ranking),
            Gamestate::new(),
        ),
        c_time
    );

    loop {
        let (score, _) = play_memory_agents(&mut bfs100, &mut uct100);
        match score.partial_cmp(&0) {
            Some(Ordering::Greater) => println!("second,loss"),
            Some(Ordering::Less) => println!("second,win"),
            Some(Ordering::Equal) => println!("second,tie"),
            _ => panic!("wtf"),
        };
        let (score, _) = play_memory_agents(&mut uct100, &mut bfs100);
        match score.partial_cmp(&0) {
            Some(Ordering::Greater) => println!("first,win"),
            Some(Ordering::Less) => println!("first,loss"),
            Some(Ordering::Equal) => println!("first,tie"),
            _ => panic!("wtf"),
        };
    }
}
