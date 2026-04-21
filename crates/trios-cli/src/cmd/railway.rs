//! `tri railway` — Railway parallel training orchestration
//!
//! Usage:
//!   tri railway deploy
//!   tri railway status [--job-id ID]
//!   tri railway sweep --seeds 4 --steps 1000

use anyhow::Result;

use crate::db::{Entry, Leaderboard};

pub async fn railway_deploy() -> Result<()> {
    println!("Deploying to Railway...");

    let status = std::process::Command::new("railway")
        .args(["up", "--detach"])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("Deployed to Railway successfully");
            println!("Monitor with: tri railway status");
        }
        Ok(_) => {
            println!("Deploy failed. Install Railway CLI: https://railway.app/cli");
            println!("Alternative: use TrainingClient to submit jobs remotely");
        }
        Err(_) => {
            println!("Railway CLI not found. Using remote API instead...");
            let client = TrainingClientWrapper::railway();
            match client.health().await {
                Ok(true) => println!("Railway API is healthy at {}", client.base_url),
                Ok(false) => println!("Railway API unhealthy"),
                Err(e) => println!("Cannot reach Railway API: {}", e),
            }
        }
    }

    Ok(())
}

pub async fn railway_status(job_id: Option<&str>) -> Result<()> {
    let client = TrainingClientWrapper::railway();

    if let Some(id) = job_id {
        println!("Checking job {}...", id);
        let status = client.job_status(id).await?;
        println_job_status(&status);
    } else {
        println!("Listing all Railway jobs...");
        let jobs = client.list_jobs().await?;
        if jobs.is_empty() {
            println!("No jobs found");
        } else {
            for job in &jobs {
                println_job_status(job);
            }
            println!("\n{} jobs total", jobs.len());
        }
    }

    Ok(())
}

pub async fn railway_sweep_parallel(seeds: u32, steps: u32, exp_id: Option<&str>) -> Result<()> {
    let exp_name = exp_id.unwrap_or("IGLA-SWEEP");
    println!("Parallel sweep: {} seeds, {} steps, exp={}", seeds, steps, exp_name);

    let client = TrainingClientWrapper::railway();

    if !client.health().await.unwrap_or(false) {
        println!("Railway API not available. Using local parallel execution...");
        return local_parallel_sweep(seeds, steps, exp_name).await;
    }

    let mut job_ids = Vec::new();

    for seed in 0..seeds {
        let job_name = format!("{}-seed-{}", exp_name, seed);
        println!("Submitting job: {}", job_name);

        let config = job_config(steps, seed);
        let script = format!("tri run {} --seeds 1", job_name);

        match client.start_job(&job_name, &script, &config).await {
            Ok(resp) => {
                println!("  Submitted: job_id={}", resp.job_id);
                job_ids.push((seed, resp.job_id));
            }
            Err(e) => {
                println!("  Failed to submit seed {}: {}", seed, e);
            }
        }
    }

    println!("\n{} jobs submitted. Polling for results...", job_ids.len());

    poll_and_collect(&client, &job_ids).await?;

    Ok(())
}

async fn local_parallel_sweep(seeds: u32, steps: u32, exp_name: &str) -> Result<()> {
    println!("Running {} seeds locally in parallel...", seeds);

    let mut handles = Vec::new();

    for seed in 0..seeds {
        let exp_id = format!("{}-seed-{}", exp_name, seed);
        let steps_arg = steps.to_string();

        let handle = std::thread::spawn(move || {
            let output = std::process::Command::new("target/debug/trios-igla-trainer")
                .args(["--exp-id", &exp_id, "--seed", &seed.to_string(), "--steps", &steps_arg])
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    Some((exp_id, stdout.to_string(), true))
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    Some((exp_id, stderr.to_string(), false))
                }
                Err(e) => {
                    Some((exp_id, e.to_string(), false))
                }
            }
        });

        handles.push(handle);
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Some((exp_id, output, success)) = handle.join().unwrap_or(None) {
            if success {
                let bpb = extract_bpb_from_output(&output);
                println!("  {} -> bpb={:.4}", exp_id, bpb.unwrap_or(0.0));
                results.push((exp_id, bpb));
            } else {
                println!("  {} -> FAILED", exp_id);
            }
        }
    }

    if !results.is_empty() {
        results.sort_by(|a, b| a.1.unwrap_or(f64::MAX).partial_cmp(&b.1.unwrap_or(f64::MAX)).unwrap());
        println!("\n=== SWEEP RESULTS ===");
        for (i, (exp_id, bpb)) in results.iter().enumerate() {
            println!(
                "  {}. {} -> bpb={:.4}{}",
                i + 1,
                exp_id,
                bpb.unwrap_or(0.0),
                if i == 0 { " <- WINNER" } else { "" }
            );
        }

        let lb = Leaderboard::open()?;
        for (exp_id, bpb) in &results {
            if let Some(bpb_val) = bpb {
                let entry = Entry {
                    id: None,
                    agent: "LOCAL".to_string(),
                    exp_id: exp_id.clone(),
                    config: format!("steps={}", steps),
                    train_bpb: *bpb_val,
                    val_bpb: *bpb_val,
                    params: 0,
                    time_sec: 0.0,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                lb.insert(&entry)?;
            }
        }
        println!("\nResults saved to leaderboard");
    }

    Ok(())
}

fn extract_bpb_from_output(output: &str) -> Option<f64> {
    for line in output.lines() {
        if line.contains("val_bpb") || line.contains("validation BPB") {
            if let Some(v) = line.split(':').nth(1).and_then(|s| s.trim().parse().ok()) {
                return Some(v);
            }
        }
    }
    None
}

async fn poll_and_collect(client: &TrainingClientWrapper, job_ids: &[(u32, String)]) -> Result<()> {
    let mut completed = Vec::new();
    let mut remaining: Vec<(u32, String)> = job_ids.to_vec();

    for _ in 0..120 {
        let mut still_running = Vec::new();

        for (seed, job_id) in &remaining {
            match client.job_status(job_id).await {
                Ok(status) => match status.status.as_str() {
                    "Completed" | "completed" => {
                        if let Ok(result) = client.job_result(job_id).await {
                            println!("  seed {} DONE: bpb={:.4} ({:.1}s)", seed, result.final_bpb, result.training_time_secs);
                            completed.push((*seed, result));
                        }
                    }
                    "Failed" | "failed" => {
                        println!("  seed {} FAILED", seed);
                    }
                    _ => {
                        still_running.push((*seed, job_id.clone()));
                    }
                },
                Err(e) => {
                    println!("  seed {}: status error: {}", seed, e);
                    still_running.push((*seed, job_id.clone()));
                }
            }
        }

        remaining = still_running;
        if remaining.is_empty() {
            break;
        }

        println!("{} jobs still running...", remaining.len());
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    }

    if !completed.is_empty() {
        completed.sort_by(|a, b| a.1.final_bpb.partial_cmp(&b.1.final_bpb).unwrap());
        println!("\n=== RAILWAY SWEEP RESULTS ===");
        for (i, (seed, result)) in completed.iter().enumerate() {
            println!(
                "  {}. seed {} -> bpb={:.4} ({:.1}s){}",
                i + 1,
                seed,
                result.final_bpb,
                result.training_time_secs,
                if i == 0 { " <- WINNER" } else { "" }
            );
        }
    }

    Ok(())
}

fn job_config(steps: u32, seed: u32) -> serde_json::Value {
    serde_json::json!({
        "num_gpus": 1,
        "gpu_type": "T4",
        "max_time_minutes": 60,
        "model_config": {},
        "hyperparams": {
            "learning_rate": 0.003,
            "batch_size": 32,
            "iterations": steps,
            "optimizer": "muon",
            "weight_decay": 0.0,
            "warmup_steps": 100,
            "schedule": "phi"
        },
        "dataset": {
            "name": "fineweb",
            "seq_len": 256,
            "vocab_size": 256
        },
        "seed": seed
    })
}

fn println_job_status(status: &JobStatusDisplay) {
    println!(
        "  [{}] {} ({})",
        status.status, status.job_id, status.name
    );
}

struct TrainingClientWrapper {
    client: reqwest::Client,
    base_url: String,
}

impl TrainingClientWrapper {
    fn railway() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            base_url: "https://trinity-training.up.railway.app".to_string(),
        }
    }

    async fn health(&self) -> Result<bool> {
        let resp = self.client.get(format!("{}/health", self.base_url)).send().await;
        match resp {
            Ok(r) => Ok(r.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    async fn start_job(&self, name: &str, script: &str, config: &serde_json::Value) -> Result<JobStatusDisplay> {
        let resp = self.client
            .post(format!("{}/api/jobs", self.base_url))
            .json(&serde_json::json!({"name": name, "script": script, "config": config}))
            .send()
            .await?;
        let body: serde_json::Value = resp.json().await?;
        Ok(JobStatusDisplay {
            job_id: body["job_id"].as_str().unwrap_or("unknown").to_string(),
            name: name.to_string(),
            status: body["status"].as_str().unwrap_or("unknown").to_string(),
        })
    }

    async fn job_status(&self, job_id: &str) -> Result<JobStatusDisplay> {
        let resp = self.client
            .get(format!("{}/api/jobs/{}", self.base_url, job_id))
            .send()
            .await?;
        let body: serde_json::Value = resp.json().await?;
        Ok(JobStatusDisplay {
            job_id: job_id.to_string(),
            name: body["name"].as_str().unwrap_or("").to_string(),
            status: body["status"].as_str().unwrap_or("unknown").to_string(),
        })
    }

    async fn job_result(&self, job_id: &str) -> Result<JobResultDisplay> {
        let resp = self.client
            .get(format!("{}/api/jobs/{}/result", self.base_url, job_id))
            .send()
            .await?;
        let body: serde_json::Value = resp.json().await?;
        Ok(JobResultDisplay {
            final_bpb: body["final_bpb"].as_f64().unwrap_or(0.0),
            training_time_secs: body["training_time_secs"].as_f64().unwrap_or(0.0),
        })
    }

    async fn list_jobs(&self) -> Result<Vec<JobStatusDisplay>> {
        let resp = self.client
            .get(format!("{}/api/jobs", self.base_url))
            .send()
            .await?;
        let body: Vec<serde_json::Value> = resp.json().await.unwrap_or_default();
        Ok(body.iter().map(|j| JobStatusDisplay {
            job_id: j["job_id"].as_str().unwrap_or("").to_string(),
            name: j["name"].as_str().unwrap_or("").to_string(),
            status: j["status"].as_str().unwrap_or("").to_string(),
        }).collect())
    }
}

struct JobStatusDisplay {
    job_id: String,
    name: String,
    status: String,
}

struct JobResultDisplay {
    final_bpb: f64,
    training_time_secs: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bpb_val_bpb() {
        let output = "step 100 val_bpb: 5.8711\n";
        assert_eq!(extract_bpb_from_output(output), Some(5.8711));
    }

    #[test]
    fn test_extract_bpb_validation() {
        let output = "validation BPB: 7.7731\n";
        assert_eq!(extract_bpb_from_output(output), Some(7.7731));
    }

    #[test]
    fn test_extract_bpb_none() {
        assert_eq!(extract_bpb_from_output("no data"), None);
    }

    #[test]
    fn test_job_config_serialization() {
        let config = job_config(1000, 42);
        assert_eq!(config["hyperparams"]["iterations"], 1000);
        assert_eq!(config["seed"], 42);
    }
}
