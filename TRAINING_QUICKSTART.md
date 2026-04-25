# CPU N-Gram Training — Quick Start

## Build
```bash
cargo build --release -p trios-train-cpu --bin ngram_train
cargo build --release --bin tri
```

## Quick Test (2K steps)
```bash
tri train --seeds 42 --steps 2000 --hidden 128 --lr 0.004
```

## Full Production Run (12K steps, 3 seeds)
```bash
tri train --seeds 42,43,44 --steps 12000 --hidden 128 --lr 0.004 --parallel
```

## Hyperparameter Sweep
```bash
# Hidden dimension sweep
tri train --seeds 42 --steps 5000 --hidden 64
tri train --seeds 42 --steps 5000 --hidden 128
tri train --seeds 42 --steps 5000 --hidden 192
```

## Model Architecture
- vocab: 128 (byte-level)
- dim: 64
- hidden: 128
- seq: 64
- ctx: 2 (4-gram)
- activation: ReLU
- lr: 0.004
- optimizer: Adam (β1=0.7/φ, β2=0.999)
- wd: 0.04

## Data
- tinyshakespeare.txt (1.1MB, 1.1M chars)
- 90% train / 10% val split

## Output Format
```
seed=42 bpb=X.XXXX time=XXs
```
