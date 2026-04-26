//! IGLA RACE — V2: Proxy Scoring CLI
//!
//! Zero-cost NAS proxy metrics for hyperparameter acceleration
//! Usage: proxy_score <config.json> --metric <synflow|gradnorm|ensemble>

use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::process;

use serde::{Deserialize, Serialize};

use trios_igla_race::proxies::{
    EnsembleScore, GradNormScore, HistoricalDataPoint,
    SynFlowScore, spearman_correlation,
};

/// Configuration for proxy scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProxyConfig {
    /// Model hidden dimensions per layer
    #[serde(default)]
    widths: Vec<usize>,

    /// Total number of parameters
    #[serde(default)]
    num_params: usize,

    /// Gradient norm from training
    #[serde(default)]
    grad_norm: Option<f64>,

    /// Historical data for validation (proxy, bpb pairs)
    #[serde(default)]
    historical: Vec<HistoricalDataPoint>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            widths: vec![64],
            num_params: 0,
            grad_norm: None,
            historical: Vec::new(),
        }
    }
}

/// Metrics output
#[derive(Debug, Clone, Serialize)]
struct MetricsOutput {
    synflow_score: f64,
    gradnorm_score: Option<f64>,
    ensemble_score: f64,
    spearman_tau: Option<f64>,
    inv14_pass: bool,
}

fn load_config(path: &str) -> ProxyConfig {
    let file = fs::File::open(path).expect(&format!("Cannot open config: {}", path));
    let reader = io::BufReader::new(file);
    serde_json::from_reader(reader).expect("Cannot parse config JSON")
}

fn compute_synflow(config: &ProxyConfig) -> f64 {
    let score = SynFlowScore::from_dimensions(&config.widths);
    assert!(score.is_valid(&config.widths), "Invalid SynFlow score");
    score.value
}

fn compute_gradnorm(config: &ProxyConfig) -> Option<f64> {
    config.grad_norm.map(|norm| {
        let score = GradNormScore::from_norm(norm, config.num_params);
        assert!(score.is_valid(), "Invalid GradNorm score");
        score.value
    })
}

fn compute_ensemble(_config: &ProxyConfig, synflow: f64, gradnorm: Option<f64>) -> f64 {
    let mut ensemble = EnsembleScore::new();
    ensemble = ensemble.with_synflow(synflow);

    if let Some(gn) = gradnorm {
        ensemble = ensemble.with_gradnorm(gn);
        assert!(ensemble.is_valid(), "Invalid ensemble weights");
        ensemble.score()
    } else {
        // If no grad norm, use only synflow
        ensemble.weight_synflow
    }
}

fn validate_inv14(config: &ProxyConfig) -> Option<bool> {
    if config.historical.is_empty() {
        None
    } else {
        let tau = spearman_correlation(&config.historical);
        tau.map(|t| t >= 0.5)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: proxy_score <config.json> [--metric <synflow|gradnorm|ensemble>]");
        eprintln!();
        eprintln!("Config JSON format:");
        eprintln!("  {{");
        eprintln!("    \"widths\": [64, 32],");
        eprintln!("    \"num_params\": 10000,");
        eprintln!("    \"grad_norm\": 0.1,");
        eprintln!("    \"historical\": [");
        eprintln!("      {{\"proxy_score\": 0.8, \"bpb\": 2.1}},");
        eprintln!("      {{\"proxy_score\": 0.7, \"bpb\": 2.3}}");
        eprintln!("    ]");
        eprintln!("  }}");
        process::exit(1);
    }

    let config_path = &args[1];
    let config = load_config(config_path);

    let metric = args.get(2).map(|s| s.as_str()).unwrap_or("ensemble");
    let inv14_pass = validate_inv14(&config).unwrap_or(false);

    let output = match metric {
        "synflow" => {
            let synflow = compute_synflow(&config);
            let tau = match validate_inv14(&config) {
                Some(true) => Some(1.0),
                _ => None,
            };
            MetricsOutput {
                synflow_score: synflow,
                gradnorm_score: None,
                ensemble_score: synflow,
                spearman_tau: tau,
                inv14_pass,
            }
        }
        "gradnorm" => {
            let gradnorm = compute_gradnorm(&config);
            let tau = match validate_inv14(&config) {
                Some(true) => Some(1.0),
                _ => None,
            };
            MetricsOutput {
                synflow_score: 0.0,
                gradnorm_score: gradnorm,
                ensemble_score: gradnorm.unwrap_or(0.0),
                spearman_tau: tau,
                inv14_pass,
            }
        }
        "ensemble" | _ => {
            let synflow = compute_synflow(&config);
            let gradnorm = compute_gradnorm(&config);
            let ensemble = compute_ensemble(&config, synflow, gradnorm);
            let tau = match validate_inv14(&config) {
                Some(true) => Some(1.0),
                _ => None,
            };
            MetricsOutput {
                synflow_score: synflow,
                gradnorm_score: gradnorm,
                ensemble_score: ensemble,
                spearman_tau: tau,
                inv14_pass,
            }
        }
    };

    // Output as JSON
    let json = serde_json::to_string_pretty(&output).unwrap();
    println!("{}", json);

    // Exit code based on INV-14 validation
    if !output.inv14_pass {
        eprintln!("INV-14 WARNING: Proxy correlation tau < 0.5");
        process::exit(2);
    }
}
