mod mechanics;
pub mod gameplay;
pub mod agent;
pub mod mcst;

use std::cmp::Ordering;

use agent::{play_memory_agents};
use agent::implementations::{BfsMemoryAgent, UctMemoryAgent};


fn main() {
    //let _ = stdin().read_line(&mut String::new());
    // from sqrt(2)/2 to 2sqrt(2)

    let base = 2_f64.sqrt() / 2_f64;
    let unit = base / 16_f64;
    let c_time = 10;

    loop {
        for m in 0..48 {
            let lr = base + f64::from(m) * unit;

            {
                let mut bfs = BfsMemoryAgent::new(c_time);
                let mut utc = UctMemoryAgent::new(c_time, lr);
                let (score, turns) = play_memory_agents(&mut bfs, &mut utc);
                println!("{}", crate::gameplay::turns_to_str(&turns));
                match score.partial_cmp(&0) {
                    Some(Ordering::Greater) => println!("second,{},loss", lr),
                    Some(Ordering::Less) => println!("second,{},win", lr),
                    Some(Ordering::Equal) => println!("second,{},tie", lr),
                    _ => panic!("wtf"),
                }
            }

            {
                let mut bfs = BfsMemoryAgent::new(c_time);
                let mut utc = UctMemoryAgent::new(c_time, lr);
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
