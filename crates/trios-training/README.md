# trios-training

HTTP client for [trinity-training](https://github.com/gHashTag/trinity-training) Railway API — training job orchestration.

## Usage

```rust
use trios_training::{TrainingClient, JobConfig, GpuType};

let client = TrainingClient::railway();

let config = JobConfig {
    num_gpus: 8,
    gpu_type: GpuType::H100,
    max_time_minutes: 10,
    model_config: serde_json::json!({}),
    hyperparams: Hyperparams { /* ... */ },
    dataset: DatasetConfig { /* ... */ },
};

let job = client.start_job("gf16-experiment", "train_gpt.py", &config).await?;
let status = client.job_status(&job.id).await?;
```
