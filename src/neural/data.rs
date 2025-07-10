use burn::{
    data::{dataloader::batcher::Batcher, dataset::Dataset},
    prelude::*,
};

#[derive(Clone)]
pub struct DataBatcher {}

#[derive(Clone, Debug)]
pub struct DataBatch<B: Backend> {
    pub states: Tensor<B, 2, Float>,
    pub targets: Tensor<B, 2, Float>,
}

pub fn compact_to_tensor<B: Backend>(mut compact: u128, device: &B::Device) -> Tensor<B, 1> {
    let mut v = [false; 64 * 3];
    for x in 0..8 {
        for y in 0..8 {
            let remainder = compact % 3;
            compact = compact / 3;
            v[(8 * x) + y + 0] = remainder == 0;
            v[(8 * x) + y + 1] = remainder == 1;
            v[(8 * x) + y + 2] = remainder == 2;
        }
    }

    Tensor::from_data(v, device)
}

impl<B: Backend> Batcher<B, (u128, f32), DataBatch<B>> for DataBatcher {
    fn batch(&self, items: Vec<(u128, f32)>, device: &B::Device) -> DataBatch<B> {
        let states = items
            .iter()
            .map(|(compact, _)| -> Tensor<B, 1> {compact_to_tensor(*compact, device)})
            .map(|t| -> Tensor<B, 2> {t.reshape([1, 64 * 3])})
            .collect();

        let targets = items
            .iter()
            .map(|(_, win_rate)| {Tensor::<B, 1, Float>::from_data([*win_rate * 2.0 - 1.0], device)})
            .map(|t| -> Tensor<B, 2> {t.reshape([1, 1])})
            .collect();

        let states = Tensor::cat(states, 0);
        let targets = Tensor::cat(targets, 0);

        DataBatch { states, targets }
    }
}

pub struct DataDataset {
    pub data: Vec<(u128, f32)>,
}

impl Dataset<(u128, f32)> for DataDataset {
    fn get(&self, index: usize) -> Option<(u128, f32)> { self.data.get(index).cloned()
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

