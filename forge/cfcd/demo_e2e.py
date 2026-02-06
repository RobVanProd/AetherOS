#!/usr/bin/env python3
"""
demo_e2e.py — End-to-end demo of the self-modifying CFC-JEPA world model.

Demonstrates the full closed-loop:
1. Load trained CFC-JEPA model (37M params)
2. Bootstrap the OS state encoder with live telemetry
3. Enter prediction loop: observe → predict → compare → learn
4. Watch the model update its own weights in real-time

Usage:
    python demo_e2e.py --checkpoint /path/to/model_final.pt
    python demo_e2e.py --checkpoint /path/to/model_final.pt --bootstrap-seconds 30
    python demo_e2e.py --checkpoint /path/to/model_final.pt --skip-bootstrap
"""

import argparse
import signal
import sys
import time

import torch

# Add model source to path
MODEL_SRC = "/home/rob/jepaworlddiffusionlm/internal_world_model"
sys.path.insert(0, MODEL_SRC)

from config import CFCDConfig
from os_state_encoder import OSStateEncoder, OSStateVector
from online_learner import OnlineLearner, OSEncoderBootstrap
from weight_manager import WeightManager
from models.cfc_jepa_world_model import CFCJEPAWorldModel, CFCJEPAConfig

import json
import os


def load_model(checkpoint_path: str, device: torch.device):
    """Load CFC-JEPA from checkpoint."""
    print(f"Loading checkpoint: {checkpoint_path}")
    ckpt = torch.load(checkpoint_path, map_location=device, weights_only=False)

    ckpt_dir = os.path.dirname(checkpoint_path)
    summary_path = os.path.join(ckpt_dir, "training_summary.json")
    ckpt_args = None

    if os.path.exists(summary_path):
        with open(summary_path) as f:
            summary = json.load(f)
            ckpt_args = summary.get("args", {})
    elif "args" in ckpt:
        ckpt_args = ckpt["args"]

    if ckpt_args:
        config = CFCJEPAConfig(
            encoder_dim=ckpt_args.get("encoder_dim", 1024),
            hidden_dim=ckpt_args.get("hidden_dim", 512),
            embed_dim=ckpt_args.get("embed_dim", 1536),
            num_layers=ckpt_args.get("num_layers", 4),
            num_heads=ckpt_args.get("num_heads", 8),
            diffusion_steps=ckpt_args.get("diffusion_steps", 10),
            use_cfc=ckpt_args.get("use_cfc", True),
        )
    else:
        config = CFCJEPAConfig(diffusion_steps=10)

    model = CFCJEPAWorldModel(config).to(device)
    state_dict = ckpt.get("model_state_dict", ckpt)
    filtered = {k: v for k, v in state_dict.items() if "gate_values" not in k}
    model.load_state_dict(filtered, strict=False)
    model.eval()

    param_count = sum(p.numel() for p in model.parameters())
    print(f"Model loaded: {param_count:,} parameters on {device}")
    return model, config


def print_gate_stats(model):
    """Print CFC gate statistics."""
    try:
        stats = model.get_gate_stats()
        for layer, vals in stats.items():
            mean, std, vmin, vmax = vals
            print(f"    Gate {layer}: mean={mean:.4f} std={std:.4f} range=[{vmin:.4f}, {vmax:.4f}]")
    except Exception:
        pass


def run_demo(args):
    # Device
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"\n{'='*60}")
    print(f"  AetherOS Phase 6: Self-Modifying CFC-JEPA World Model")
    print(f"{'='*60}")
    print(f"  Device: {device}")

    # Load model
    model, model_config = load_model(args.checkpoint, device)

    # OS encoder
    os_encoder = OSStateEncoder(input_dim=128, output_dim=1024).to(device)
    os_encoder.eval()
    enc_params = sum(p.numel() for p in os_encoder.parameters())
    print(f"  OS Encoder: {enc_params:,} parameters")

    # Weight manager
    weight_dir = os.path.expanduser("~/.aether/aurora/models")
    os.makedirs(weight_dir, exist_ok=True)
    weight_manager = WeightManager(model, weight_dir, max_versions=10)

    # Online learner
    learner = OnlineLearner(
        model=model,
        os_encoder=os_encoder,
        weight_manager=weight_manager,
        device=device,
        online_lr=1e-5,
        warmup_lr=1e-6,
        warmup_updates=100,
        grad_clip_norm=1.0,
        prediction_error_threshold=0.5,
        buffer_size=64,
        min_buffer_for_update=16,
    )
    learner.enabled = True

    # Bootstrap OS encoder
    if not args.skip_bootstrap:
        print(f"\n--- OS Encoder Bootstrap ({args.bootstrap_seconds}s) ---")
        bootstrap = OSEncoderBootstrap(
            encoder=os_encoder,
            device=device,
            collection_seconds=args.bootstrap_seconds,
            sample_hz=10,
            epochs=50,
        )
        telemetry = bootstrap.collect_telemetry(
            callback=lambda i, n: print(f"  Collected {i}/{n} samples")
        )
        result = bootstrap.train_encoder(telemetry)
        print(f"  Bootstrap done: accuracy={result.get('final_accuracy', 0)*100:.1f}%")
    else:
        print("\n  (Skipping bootstrap)")

    # Signal handler for clean shutdown
    running = [True]

    def handler(sig, frame):
        print("\n\nShutting down...")
        running[0] = False

    signal.signal(signal.SIGINT, handler)

    # Prediction loop
    print(f"\n{'='*60}")
    print(f"  Starting closed-loop prediction (interval={args.interval}s)")
    print(f"  Learning threshold: {learner.prediction_error_threshold}")
    print(f"  Press Ctrl+C to stop")
    print(f"{'='*60}\n")

    iteration = 0
    while running[0]:
        iteration += 1
        result = learner.observe_and_learn(interval_sec=args.interval)

        error = result["prediction_error"]
        updated = result["updated"]
        buf_size = result["buffer_size"]
        total_updates = result["total_updates"]
        lr = result["lr"]

        # Status line
        status = "UPDATE" if updated else "observe"
        loss_str = f" loss={result['update_loss']:.4f}" if updated else ""
        version = weight_manager.get_current_version()

        print(f"  [{iteration:4d}] error={error:.4f} | {status}{loss_str} | "
              f"buf={buf_size}/64 | updates={total_updates} | "
              f"lr={lr:.1e} | weights={version}")

        # Print gate stats every 10 iterations
        if iteration % 10 == 0:
            print_gate_stats(model)

        # Save weights every 50 updates
        if updated and total_updates % 50 == 0:
            v = weight_manager.save_version(
                metrics={"prediction_error": error, "update_loss": result["update_loss"]}
            )
            print(f"  >>> Saved weight version: {v}")

    # Final stats
    stats = learner.get_stats()
    print(f"\n{'='*60}")
    print(f"  Session Summary")
    print(f"{'='*60}")
    print(f"  Total observations: {stats['total_observations']}")
    print(f"  Total weight updates: {stats['total_updates']}")
    print(f"  Update rate: {stats['update_rate']*100:.1f}%")
    print(f"  Mean prediction error: {stats['mean_prediction_error']:.4f}")
    print(f"  Final weight version: {weight_manager.get_current_version()}")
    print(f"  Learning rate: {stats['current_lr']:.1e}")

    if stats['recent_errors']:
        first_5 = stats['recent_errors'][:5]
        last_5 = stats['recent_errors'][-5:]
        print(f"  First 5 errors: {[round(e, 4) for e in first_5]}")
        print(f"  Last 5 errors:  {[round(e, 4) for e in last_5]}")

    print()


def main():
    parser = argparse.ArgumentParser(description="AetherOS Self-Modifying AI Demo")
    parser.add_argument("--checkpoint", type=str, required=True,
                        help="Path to CFC-JEPA checkpoint (.pt)")
    parser.add_argument("--interval", type=float, default=1.0,
                        help="Seconds between observations (default: 1.0)")
    parser.add_argument("--bootstrap-seconds", type=int, default=30,
                        help="Seconds of telemetry for bootstrap (default: 30)")
    parser.add_argument("--skip-bootstrap", action="store_true",
                        help="Skip encoder bootstrap (use random encoder)")
    args = parser.parse_args()
    run_demo(args)


if __name__ == "__main__":
    main()
