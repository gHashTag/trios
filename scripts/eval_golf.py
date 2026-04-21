#!/usr/bin/env python3
"""
eval_golf.py — Parameter Golf evaluation script

Computes tokenizer-agnostic bits-per-byte (BPB) on FineWeb validation data
using sliding-window evaluation with configurable stride.

Usage:
    python3 eval_golf.py --checkpoint best_model.pt --data fineweb_val.bin
    python3 eval_golf.py --checkpoint best_model.pt --data tinyshakespeare.txt --stride 64
"""

import argparse
import math
import os

import torch
import torch.nn.functional as F


def load_config_from_checkpoint(path):
    state = torch.load(path, map_location="cpu", weights_only=False)
    tok_emb = state.get("tok_emb.weight")
    if tok_emb is None:
        raise ValueError("Invalid checkpoint: no tok_emb.weight")
    vocab_size, d_model = tok_emb.shape

    n_layers = 0
    for key in state:
        if key.startswith("blocks.") and ".attn.wqkv.weight" in key:
            layer_idx = int(key.split(".")[1])
            n_layers = max(n_layers, layer_idx + 1)

    wqkv = state.get("blocks.0.attn.wqkv.weight")
    if wqkv is None:
        raise ValueError("Cannot determine head config")
    total_qkv = wqkv.shape[0]
    head_dim_candidates = [8, 16, 24, 32, 48, 64, 96]
    n_heads = 0
    head_dim = 0
    for hd in head_dim_candidates:
        nh = total_qkv // (3 * hd)
        if nh * 3 * hd == total_qkv and nh * hd == d_model:
            n_heads, head_dim = nh, hd
            break
    if n_heads == 0:
        n_heads = 8
        head_dim = d_model // n_heads

    return {
        "vocab_size": vocab_size,
        "d_model": d_model,
        "n_heads": n_heads,
        "head_dim": head_dim,
        "n_layers": n_layers,
        "ff_mult": 4,
    }


def build_model_from_config(cfg_dict, checkpoint_path):
    import train_gpt
    cfg = train_gpt.Config()
    cfg.vocab_size = cfg_dict["vocab_size"]
    cfg.d_model = cfg_dict["d_model"]
    cfg.n_heads = cfg_dict["n_heads"]
    cfg.head_dim = cfg_dict["head_dim"]
    cfg.n_layers = cfg_dict["n_layers"]
    cfg.ff_mult = cfg_dict.get("ff_mult", 4)

    model = train_gpt.GPTModel(cfg)
    state = torch.load(checkpoint_path, map_location="cpu", weights_only=False)
    model.load_state_dict(state)
    return model, cfg


@torch.no_grad()
def evaluate_bpb_sliding(model, data, cfg, device, stride=64, max_chunks=1000):
    model.eval()
    total_bits = 0.0
    total_bytes = 0
    seq = cfg.seq_len

    pos = 0
    chunks = 0
    while pos + seq + 1 <= len(data) and chunks < max_chunks:
        chunk = data[pos:pos + seq + 1]
        if len(chunk) < seq + 1:
            break
        x = torch.tensor([chunk], dtype=torch.long, device=device)

        if pos == 0:
            start_tok = 0
        else:
            start_tok = stride

        logits = model(x[:, :-1])
        targets = x[:, 1:]
        log_probs = F.log_softmax(logits.float(), dim=-1)
        target_log_probs = log_probs.gather(2, targets.unsqueeze(-1)).squeeze(-1)

        if pos > 0:
            target_log_probs[:, :start_tok] = 0.0

        total_bits += -target_log_probs.sum().item() / math.log(2)
        total_bytes += target_log_probs.shape[1] - (0 if pos == 0 else start_tok)

        pos += stride
        chunks += 1

    model.train()
    return total_bits / max(total_bytes, 1)


def main():
    parser = argparse.ArgumentParser(description="Parameter Golf Evaluation")
    parser.add_argument("--checkpoint", required=True, help="Path to model checkpoint")
    parser.add_argument("--data", required=True, help="Path to evaluation data")
    parser.add_argument("--stride", type=int, default=64, help="Sliding window stride")
    parser.add_argument("--seq-len", type=int, default=None)
    parser.add_argument("--max-chunks", type=int, default=1000)
    args = parser.parse_args()

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    cfg_dict = load_config_from_checkpoint(args.checkpoint)
    model, cfg = build_model_from_config(cfg_dict, args.checkpoint)
    model.to(device)

    if args.seq_len:
        cfg.seq_len = args.seq_len

    print(f"Model: {cfg.n_layers}L d={cfg.d_model} h={cfg.n_heads}x{cfg.head_dim}")
    print(f"Params: {cfg.param_count():,} ({cfg.model_size_mb_fp16():.2f} MB FP16)")

    with open(args.data, "rb") as f:
        raw = f.read()
    data = list(raw)
    split = int(len(data) * 0.9)
    val_data = data[split:]
    print(f"Val data: {len(val_data):,} bytes")

    bpb = evaluate_bpb_sliding(model, val_data, cfg, device, args.stride, args.max_chunks)
    print(f"\nval_bpb: {bpb:.4f}")
    print(f"params: {cfg.param_count()}")
    print(f"stride: {args.stride}")


if __name__ == "__main__":
    main()
