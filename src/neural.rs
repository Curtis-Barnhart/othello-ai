pub mod data;
pub mod model_a;
pub mod model_b;

use burn::{
    data::dataset::InMemDataset,
    prelude::{Backend, Module}, tensor::{Tensor}
};

use crate::{
    agent::Agent,
    gameplay::{Gamestate, Turn},
    neural::data::compact_to_tensor,
};

fn create_artifact_dir(artifact_dir: &str) {
    // Remove existing artifacts before to get an accurate learner summary
    std::fs::remove_dir_all(artifact_dir).ok();
    std::fs::create_dir_all(artifact_dir).ok();
}

fn get_train_data() -> InMemDataset<(u128, f32)> {
    InMemDataset::<(u128, f32)>::from_csv("train.csv", &csv::ReaderBuilder::new()).unwrap()
}

fn get_validation_data() -> InMemDataset<(u128, f32)> {
    InMemDataset::<(u128, f32)>::from_csv("valid.csv", &csv::ReaderBuilder::new()).unwrap()
}

pub trait StaticNeuralEval {
    type B: Backend;

    fn eval(&self, tensor: Tensor<Self::B, 1>) -> f32;
}

pub struct ModuleAgent<M, B>
where
    B: Backend,
    M: Module<B>
{
    module: M,
    device: B::Device,
}

impl<M, B> ModuleAgent<M, B>
where
    B: Backend,
    // Not sure that I fully understand the B = B. Will have to come back to this later.
    M: Module<B> + StaticNeuralEval<B = B>
{
    pub fn new(module: M, device: B::Device) -> Self {
        ModuleAgent {
            module,
            device,
        }
    }

    fn eval_state(&self, state: &Gamestate) -> f32 {
        let in_tensor = compact_to_tensor::<B>(state.board().to_compact(), &self.device);
        self.module.eval(in_tensor)
    }
}

impl<M, B> Agent for ModuleAgent<M, B>
where
    B: Backend,
    M: Module<B> + StaticNeuralEval<B = B>
{
    fn make_move(&self, state: &Gamestate) -> Turn {
        let moves = state.get_moves();
        let games = moves
            .iter()
            .map(|t: &Turn| {
                let mut next = state.clone();
                next.make_move_fast(*t);
                self.eval_state(&next)
            });
        *moves.iter()
            .zip(games)
            .max_by(|(_t1, value1), (_t2, value2)| {
                value1.total_cmp(value2)
            })
            .expect("Given a game with no moves")
            .0
    }
}
