use burn::{
    data::{dataloader::DataLoaderBuilder, dataset::InMemDataset},
    nn::{loss::MseLoss, Dropout, DropoutConfig, Linear, LinearConfig, Relu},
    optim::AdamConfig,
    prelude::*,
    record::CompactRecorder,
    tensor::backend::AutodiffBackend,
    train::{
        metric::LossMetric,
        LearnerBuilder, RegressionOutput, TrainOutput, TrainStep, ValidStep
    }
};

use super::data::{DataBatch, DataBatcher};

#[derive(Config, Debug)]
pub struct ModelConfig {
    #[config(default = "0.3")]
    dropout: f64,
}

impl ModelConfig {
    /// Returns the initialized model.
    pub fn init<B: Backend>(&self, device: &B::Device) -> Model<B> {
        Model {
            dropout: DropoutConfig::new(self.dropout).init(),
            linear1: LinearConfig::new(64 * 3, 2048).init(device),
            linear2: LinearConfig::new(2048, 2048).init(device),
            linear3: LinearConfig::new(2048, 2048).init(device),
            linear4: LinearConfig::new(2048, 2048).init(device),
//            linear5: LinearConfig::new(2048, 2048).init(device),
//            linear6: LinearConfig::new(2048, 2048).init(device),
            activation: Relu::new(),
        }
    }
}

#[derive(Module, Debug)]
pub struct Model<B: Backend> {
    dropout: Dropout,
    linear1: Linear<B>,
    linear2: Linear<B>,
    linear3: Linear<B>,
    linear4: Linear<B>,
//    linear5: Linear<B>,
//    linear6: Linear<B>,
    activation: Relu,
}

impl<B: Backend> Model<B> {
    /// # Shapes
    ///   - Images [batch_size, coords]
    ///   - Output [batch_size, num_classes]
    pub fn forward(&self, states: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = self.linear1.forward(states);
        let x = self.dropout.forward(x);

        let x = self.activation.forward(x);
        let x = self.linear2.forward(x);
        let x = self.dropout.forward(x);

        let x = self.activation.forward(x);
        let x = self.linear3.forward(x);
        let x = self.dropout.forward(x);

        let x = self.activation.forward(x);
        let x = self.linear4.forward(x);
        let x = self.dropout.forward(x);

//        let x = self.activation.forward(x);
//        let x = self.linear5.forward(x);
//        let x = self.dropout.forward(x);
//
//        let x = self.activation.forward(x);
//        let x = self.linear6.forward(x);
//        let x = self.dropout.forward(x);
        x
    }

    pub fn forward_step(
        &self,
        states: Tensor<B, 2>,
        targets: Tensor<B, 2, Float>,
    ) -> RegressionOutput<B> {
        let output = self.forward(states);
        let loss = MseLoss::new()
            .forward(output.clone(), targets.clone(), nn::loss::Reduction::Mean);

        RegressionOutput::new(loss, output, targets)
    }
}

impl<B: AutodiffBackend> TrainStep<DataBatch<B>, RegressionOutput<B>> for Model<B> {
    fn step(&self, batch: DataBatch<B>) -> TrainOutput<RegressionOutput<B>> {
        let item = self.forward_step(batch.states, batch.targets);

        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> ValidStep<DataBatch<B>, RegressionOutput<B>> for Model<B> {
    fn step(&self, batch: DataBatch<B>) -> RegressionOutput<B> {
        self.forward_step(batch.states, batch.targets)
    }
}

#[derive(Config)]
pub struct TrainingConfig {
    pub model: ModelConfig,
    pub optimizer: AdamConfig,
    #[config(default = 8)]
    pub num_epochs: usize,
    #[config(default = 64)]
    pub batch_size: usize,
    #[config(default = 8)]
    pub num_workers: usize,
    #[config(default = 42)]
    pub seed: u64,
    #[config(default = 1.0e-4)]
    pub learning_rate: f64,
}

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

pub fn train<B: AutodiffBackend>(artifact_dir: &str, config: TrainingConfig, device: B::Device) {
    create_artifact_dir(artifact_dir);
    config.save(format!("{artifact_dir}/config.json"))
        .expect("Config should be saved successfully");

    B::seed(config.seed);

    let batcher = DataBatcher {};

    let dataloader_train = DataLoaderBuilder::new(batcher.clone())
        .batch_size(config.batch_size)
        .shuffle(config.seed)
        .num_workers(config.num_workers)
        .build(get_train_data());

    let dataloader_test = DataLoaderBuilder::new(batcher)
        .batch_size(config.batch_size)
        .shuffle(config.seed)
        .num_workers(config.num_workers)
        .build(get_validation_data());

    let learner = LearnerBuilder::new(artifact_dir)
        .metric_train_numeric(LossMetric::new())
        .metric_valid_numeric(LossMetric::new())
        .with_file_checkpointer(CompactRecorder::new())
        //.checkpoint(8)
        .devices(vec![device.clone()])
        .num_epochs(config.num_epochs)
        .summary()
        .build(
            config.model.init::<B>(&device),
            config.optimizer.init(),
            config.learning_rate,
        );

    let model_trained = learner.fit(dataloader_train, dataloader_test);

    model_trained
        .save_file(format!("{artifact_dir}/model"), &CompactRecorder::new())
        .expect("Trained model should be saved successfully");
}
