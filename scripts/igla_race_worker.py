#!/usr/bin/env python3
"""
IGLA RACE WORKER
================
Non-stop distributed hyperparameter search using:
  - Optuna TPE (Bayesian) sampler
  - ASHA / Hyperband pruning (kill bad trials at step 1000)
  - Neon PostgreSQL as shared leaderboard (all machines coordinate via 1 DB)
  - Infinite loop: never stops until BPB < TARGET or Ctrl+C

Usage:
  # Machine 1 (master + workers):
  MACHINE_ID=mac-studio-1 python scripts/igla_race_worker.py

  # Machine 2 (any other machine - same command!):
  MACHINE_ID=macbook-pro-2 python scripts/igla_race_worker.py

  # Leaderboard:
  python scripts/igla_race_worker.py --status

  # Custom workers:
  python scripts/igla_race_worker.py --workers 8 --target 1.5

Dependencies:
  pip install optuna psycopg2-binary torch numpy

Neon DB URL (set in .env or env var NEON_URL):
  postgresql://neondb_owner:npg_NHBC5hdbM0Kx@ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb?sslmode=require
"""

import argparse
import math
import os
import random
import socket
import time
from datetime import datetime

import numpy as np
import optuna
import torch
import torch.nn as nn
import torch.nn.functional as F
from optuna.pruners import HyperbandPruner
from optuna.samplers import TPESampler

# ---------------------------------------------------------------------------
# CONFIG
# ---------------------------------------------------------------------------

STUDY_NAME  = "igla_race_v1"
TARGET_BPB  = 1.50
DATA_PATH   = "data/tinyshakespeare.txt"
MACHINE_ID  = os.environ.get("MACHINE_ID", socket.gethostname())

# Neon PostgreSQL URL — set via .env or NEON_URL env var
NEON_URL = os.environ.get(
    "NEON_URL",
    "postgresql://neondb_owner:npg_NHBC5hdbM0Kx@ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb?sslmode=require&channel_binding=require"
)

# ASHA checkpoints (3^k steps — Trinity architecture!)
ASHA_RUNGS = [1000, 3000, 9000, 27000]


# ---------------------------------------------------------------------------
# MODEL (copied from igla_train.py — single source of truth)
# ---------------------------------------------------------------------------

class BigramHashEmbedding(nn.Module):
    def __init__(self, vocab_size: int, bigram_dim: int, model_dim: int):
        super().__init__()
        self.vocab_size = vocab_size
        self.embed = nn.Embedding(vocab_size, bigram_dim)
        self.proj = nn.Linear(bigram_dim, model_dim, bias=False) if bigram_dim != model_dim else None
        self.scale = nn.Parameter(torch.tensor(0.05, dtype=torch.float32))
        nn.init.zeros_(self.embed.weight)
        if self.proj is not None:
            nn.init.zeros_(self.proj.weight)

    def bigram_hash(self, tokens: torch.Tensor) -> torch.Tensor:
        t = tokens.to(torch.int32)
        mod_val = self.vocab_size - 1
        batch, seq = t.shape
        out = torch.zeros_like(t)
        out[:, 0] = mod_val
        if seq > 1:
            t_curr = t[:, 1:]
            t_prev = t[:, :-1]
            hash_val = torch.bitwise_xor(36313 * t_curr, 27191 * t_prev) % mod_val
            out[:, 1:] = hash_val
        return out.long()

    def forward(self, token_ids: torch.Tensor) -> torch.Tensor:
        h = self.embed(self.bigram_hash(token_ids))
        if self.proj is not None:
            h = self.proj(h)
        return h * self.scale


class SmearGate(nn.Module):
    def __init__(self, dim: int):
        super().__init__()
        self.gate = nn.Parameter(torch.zeros(dim, dtype=torch.float32))

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        g = torch.sigmoid(self.gate).view(1, 1, -1)
        x_prev = torch.cat([torch.zeros_like(x[:, :1]), x[:, :-1]], dim=1)
        return (1 - g) * x + g * x_prev


class RMSNorm(nn.Module):
    def __init__(self, dim: int, eps: float = 1e-6):
        super().__init__()
        self.scale = nn.Parameter(torch.ones(dim))
        self.eps = eps

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        rms = (x.square().mean(dim=-1, keepdim=True) + self.eps).sqrt()
        return x / rms * self.scale


class AttentionLayer(nn.Module):
    """Single causal self-attention layer."""
    def __init__(self, dim: int, n_heads: int = 4):
        super().__init__()
        self.n_heads = n_heads
        self.head_dim = dim // n_heads
        self.qkv = nn.Linear(dim, 3 * dim, bias=False)
        self.proj = nn.Linear(dim, dim, bias=False)
        self.norm = RMSNorm(dim)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        B, T, C = x.shape
        res = x
        x = self.norm(x)
        qkv = self.qkv(x).chunk(3, dim=-1)
        q, k, v = [t.view(B, T, self.n_heads, self.head_dim).transpose(1, 2) for t in qkv]
        scale = self.head_dim ** -0.5
        attn = (q @ k.transpose(-2, -1)) * scale
        mask = torch.triu(torch.ones(T, T, device=x.device), diagonal=1).bool()
        attn = attn.masked_fill(mask, float('-inf'))
        attn = F.softmax(attn, dim=-1)
        out = (attn @ v).transpose(1, 2).reshape(B, T, C)
        return res + self.proj(out)


class IGLARaceModel(nn.Module):
    """Extended IGLA model with optional attention layers."""
    def __init__(self, vocab_size: int, d_model: int, n_attn_layers: int,
                 use_bigram: bool, use_smear: bool, bigram_vocab: int = 729):
        super().__init__()
        self.tok_emb = nn.Embedding(vocab_size, d_model)
        self.bigram = BigramHashEmbedding(bigram_vocab, min(d_model, 64), d_model) if use_bigram else None
        self.smear  = SmearGate(d_model) if use_smear else nn.Identity()
        self.attn_layers = nn.ModuleList([AttentionLayer(d_model) for _ in range(n_attn_layers)])
        self.norm   = RMSNorm(d_model)

    def forward(self, tokens: torch.Tensor) -> torch.Tensor:
        x = self.tok_emb(tokens)
        if self.bigram is not None:
            x = x + self.bigram(tokens)
        x = self.smear(x)
        for layer in self.attn_layers:
            x = layer(x)
        x = self.norm(x)
        return x @ self.tok_emb.weight.T

    def forward_with_loss(self, tokens: torch.Tensor):
        logits = self.forward(tokens[:, :-1])
        loss = F.cross_entropy(logits.reshape(-1, logits.size(-1)), tokens[:, 1:].reshape(-1))
        return logits, loss


class MuonOptimizer:
    def __init__(self, params, lr: float, momentum: float = 0.99, weight_decay: float = 0.04):
        self.params  = list(params)
        self.lr      = lr
        self.momentum = momentum
        self.weight_decay = weight_decay
        self.buffers = [None] * len(self.params)

    def step(self):
        for i, p in enumerate(self.params):
            if p.grad is None:
                continue
            if self.weight_decay > 0:
                p.data.mul_(1 - self.lr * self.weight_decay)
            grad = p.grad
            if self.buffers[i] is None:
                self.buffers[i] = grad.clone()
            else:
                self.buffers[i].mul_(self.momentum).add_(grad, alpha=1 - self.momentum)
            p.data.add_(self.buffers[i], alpha=-self.lr)

    def zero_grad(self):
        for p in self.params:
            if p.grad is not None:
                p.grad.zero_()


# ---------------------------------------------------------------------------
# DATA
# ---------------------------------------------------------------------------

_TRAIN_TOKENS = None
_VAL_TOKENS   = None
_VOCAB_SIZE   = None

def load_data():
    global _TRAIN_TOKENS, _VAL_TOKENS, _VOCAB_SIZE
    if _TRAIN_TOKENS is not None:
        return _TRAIN_TOKENS, _VAL_TOKENS, _VOCAB_SIZE
    with open(DATA_PATH, 'r') as f:
        text = f.read()
    chars = sorted(set(text))
    _VOCAB_SIZE = len(chars)
    c2i = {c: i for i, c in enumerate(chars)}
    tokens = [c2i[c] for c in text]
    split = int(len(tokens) * 0.9)
    _TRAIN_TOKENS = tokens[:split]
    _VAL_TOKENS   = tokens[split:]
    return _TRAIN_TOKENS, _VAL_TOKENS, _VOCAB_SIZE


def get_batch(tokens, batch_size, seq_len, device):
    n = batch_size * (seq_len + 1)
    start = random.randint(0, max(0, len(tokens) - n))
    chunk = tokens[start:start + n]
    if len(chunk) < n:
        chunk = chunk + [0] * (n - len(chunk))
    return torch.tensor(chunk, dtype=torch.long, device=device).view(batch_size, seq_len + 1)


def evaluate_bpb(model, val_tokens, batch_size, seq_len, device):
    model.eval()
    total, count = 0.0, 0
    n = batch_size * (seq_len + 1)
    with torch.no_grad():
        for i in range(0, len(val_tokens) - n, n):
            batch = torch.tensor(val_tokens[i:i+n], dtype=torch.long, device=device).view(batch_size, seq_len + 1)
            _, loss = model.forward_with_loss(batch)
            total += loss.item()
            count += 1
    model.train()
    return (total / max(count, 1)) / math.log(2)


# ---------------------------------------------------------------------------
# OPTUNA OBJECTIVE
# ---------------------------------------------------------------------------

def objective(trial: optuna.Trial) -> float:
    train_tokens, val_tokens, vocab_size = load_data()

    # --- Search space ---
    d_model      = trial.suggest_categorical('d_model',   [64, 96, 128, 144, 192, 256])
    seq_len      = trial.suggest_categorical('seq_len',   [32, 64, 128, 256])
    batch_size   = trial.suggest_categorical('batch_size', [4, 8, 16])
    n_attn       = trial.suggest_int('n_attn_layers', 0, 3)
    use_bigram   = trial.suggest_categorical('use_bigram', [True, False])
    use_smear    = trial.suggest_categorical('use_smear',  [True, False])
    optimizer_name = trial.suggest_categorical('optimizer', ['adamw', 'muon'])
    lr           = trial.suggest_float('lr', 1e-4, 0.1, log=True)
    weight_decay = trial.suggest_float('weight_decay', 1e-4, 0.1, log=True)
    seed         = trial.suggest_int('seed', 0, 9999)

    # φ-based lr option
    use_phi_lr = trial.suggest_categorical('phi_lr', [True, False])
    if use_phi_lr:
        lr = 0.618 / math.sqrt(d_model)

    # Fibonacci dim option
    fib_dims = [55, 89, 144, 233, None]
    fib_choice = trial.suggest_categorical('fib_dim', [str(x) for x in fib_dims])
    if fib_choice != 'None':
        d_model = int(fib_choice)

    random.seed(seed)
    torch.manual_seed(seed)
    np.random.seed(seed)
    device = torch.device('cpu')

    model = IGLARaceModel(
        vocab_size=vocab_size,
        d_model=d_model,
        n_attn_layers=n_attn,
        use_bigram=use_bigram,
        use_smear=use_smear,
    ).to(device)

    if optimizer_name == 'muon':
        opt = MuonOptimizer(model.parameters(), lr=lr, weight_decay=weight_decay)
    else:
        opt = torch.optim.AdamW(model.parameters(), lr=lr, weight_decay=weight_decay)

    total_steps = ASHA_RUNGS[-1]
    best_bpb    = float('inf')

    for step in range(1, total_steps + 1):
        batch = get_batch(train_tokens, batch_size, seq_len, device)
        opt.zero_grad()
        _, loss = model.forward_with_loss(batch)
        loss.backward()
        if hasattr(opt, 'step'):
            opt.step()

        # ASHA checkpoint
        if step in ASHA_RUNGS:
            bpb = evaluate_bpb(model, val_tokens, batch_size, seq_len, device)
            if bpb < best_bpb:
                best_bpb = bpb

            trial.report(bpb, step)
            print(f"  [{MACHINE_ID}] trial={trial.number} step={step:>6} BPB={bpb:.4f} best={best_bpb:.4f}")

            # IGLA found!
            if bpb < TARGET_BPB:
                print(f"\n🎯🎯🎯 IGLA НАЙДЕНА! BPB={bpb:.4f} < {TARGET_BPB} at step={step} 🎯🎯🎯")
                _announce_winner(trial, bpb, step)
                return bpb

            # ASHA prune — kill bad trials early
            if trial.should_prune():
                print(f"  ✂️  PRUNED trial={trial.number} at step={step} BPB={bpb:.4f}")
                raise optuna.TrialPruned()

    return best_bpb


def _announce_winner(trial: optuna.Trial, bpb: float, step: int):
    print(f"""
╔══════════════════════════════════════════╗
║  🏆  IGLA НАЙДЕНА!                       ║
║  Machine : {MACHINE_ID:<30} ║
║  Trial   : {trial.number:<30} ║
║  BPB     : {bpb:<30.6f} ║
║  Steps   : {step:<30} ║
║  Config  : {str(trial.params)[:30]:<30} ║
╚══════════════════════════════════════════╝
    """)
    # Write winner to local file
    with open('IGLA_WINNER.md', 'w') as f:
        f.write(f"# 🏆 IGLA WINNER\n\n")
        f.write(f"- **Machine**: {MACHINE_ID}\n")
        f.write(f"- **Trial**: {trial.number}\n")
        f.write(f"- **BPB**: {bpb:.6f}\n")
        f.write(f"- **Steps**: {step}\n")
        f.write(f"- **Config**: {trial.params}\n")
        f.write(f"- **Found at**: {datetime.utcnow().isoformat()}\n")


# ---------------------------------------------------------------------------
# LEADERBOARD
# ---------------------------------------------------------------------------

def show_leaderboard(study: optuna.Study):
    trials = [t for t in study.trials if t.state == optuna.trial.TrialState.COMPLETE]
    trials.sort(key=lambda t: t.value or float('inf'))

    print(f"\n{'='*72}")
    print(f"  🏎️  IGLA RACE LEADERBOARD  |  Study: {study.study_name}")
    print(f"  Target: BPB < {TARGET_BPB}")
    print(f"  Total trials: {len(study.trials)}  |  "
          f"Complete: {len(trials)}  |  "
          f"Pruned: {sum(1 for t in study.trials if t.state == optuna.trial.TrialState.PRUNED)}")
    print(f"{'='*72}")
    print(f"  {'Rank':>4}  {'Trial':>6}  {'BPB':>8}  {'Opt':>6}  {'dim':>5}  {'attn':>5}  {'seq':>5}")
    print(f"  {'-'*60}")
    for i, t in enumerate(trials[:20]):
        bpb  = t.value or 0.0
        opt  = t.params.get('optimizer', '?')[:6]
        dim  = t.params.get('fib_dim', t.params.get('d_model', '?'))
        attn = t.params.get('n_attn_layers', 0)
        seq  = t.params.get('seq_len', '?')
        mark = " 🏆" if bpb < TARGET_BPB else ""
        print(f"  {i+1:>4}  {t.number:>6}  {bpb:>8.4f}  {opt:>6}  {str(dim):>5}  {attn:>5}  {str(seq):>5}{mark}")
    print(f"{'='*72}\n")


# ---------------------------------------------------------------------------
# MAIN RACE LOOP
# ---------------------------------------------------------------------------

def run_race(n_jobs: int = 4, target: float = TARGET_BPB):
    global TARGET_BPB
    TARGET_BPB = target

    print(f"""
╔══════════════════════════════════════════════════╗
║  🏎️  IGLA RACE STARTING                         ║
║  Machine  : {MACHINE_ID:<36} ║
║  Workers  : {n_jobs:<36} ║
║  Target   : BPB < {target:<30} ║
║  Study    : {STUDY_NAME:<36} ║
║  DB       : Neon PostgreSQL                      ║
║  Stop     : Ctrl+C (or BPB < {target})         ║
╚══════════════════════════════════════════════════╝
    """)

    # Connect to Neon PostgreSQL via Optuna
    pruner = HyperbandPruner(
        min_resource=1000,      # первый checkpoint — 1000 шагов
        max_resource=27000,     # максимум — 27000 шагов (3^k)
        reduction_factor=3,     # убиваем 2/3 худших на каждом rung
    )
    sampler = TPESampler(seed=42)  # Bayesian Tree Parzen Estimator

    study = optuna.create_study(
        study_name=STUDY_NAME,
        storage=NEON_URL,
        direction="minimize",
        load_if_exists=True,
        pruner=pruner,
        sampler=sampler,
    )

    print(f"📊 Study loaded. Existing trials: {len(study.trials)}")

    # Infinite race loop — never stops until IGLA found
    trial_count = 0
    try:
        while True:
            # Show leaderboard every 10 trials
            if trial_count % 10 == 0 and trial_count > 0:
                show_leaderboard(study)

            # Run n_jobs trials in parallel
            study.optimize(
                objective,
                n_trials=n_jobs,
                n_jobs=n_jobs,
                show_progress_bar=False,
                callbacks=[_check_winner_callback],
            )
            trial_count += n_jobs

            # Check if winner found
            completed = [t for t in study.trials if t.state == optuna.trial.TrialState.COMPLETE and t.value is not None]
            if completed:
                best_bpb = min(t.value for t in completed)
                print(f"  Best BPB so far: {best_bpb:.4f} (target: {TARGET_BPB})")
                if best_bpb < TARGET_BPB:
                    print(f"\n🎯 IGLA FOUND! BPB={best_bpb:.4f}")
                    show_leaderboard(study)
                    break

    except KeyboardInterrupt:
        print("\n⏹️  Race paused by user (Ctrl+C)")
        show_leaderboard(study)
        print("Resume with: python scripts/igla_race_worker.py")


def _check_winner_callback(study: optuna.Study, trial: optuna.trial.FrozenTrial):
    """Optuna callback: print progress after each trial."""
    if trial.state == optuna.trial.TrialState.COMPLETE and trial.value is not None:
        print(f"  ✅ Trial {trial.number} done: BPB={trial.value:.4f} [{MACHINE_ID}]")
    elif trial.state == optuna.trial.TrialState.PRUNED:
        print(f"  ✂️  Trial {trial.number} pruned [{MACHINE_ID}]")


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="IGLA RACE WORKER — distributed hyperparameter search"
    )
    parser.add_argument('--workers',  type=int,   default=4,          help='parallel trials per machine (default: 4)')
    parser.add_argument('--target',   type=float, default=TARGET_BPB, help='BPB target to stop race (default: 1.5)')
    parser.add_argument('--status',   action='store_true',            help='show leaderboard and exit')
    parser.add_argument('--study',    type=str,   default=STUDY_NAME, help='Optuna study name')
    parser.add_argument('--machine',  type=str,   default=MACHINE_ID, help='machine identifier')
    args = parser.parse_args()

    global STUDY_NAME, MACHINE_ID
    STUDY_NAME = args.study
    MACHINE_ID = args.machine

    optuna.logging.set_verbosity(optuna.logging.WARNING)

    if args.status:
        study = optuna.load_study(
            study_name=STUDY_NAME,
            storage=NEON_URL,
        )
        show_leaderboard(study)
        return

    run_race(n_jobs=args.workers, target=args.target)


if __name__ == "__main__":
    main()
