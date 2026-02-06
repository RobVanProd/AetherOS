"""
Online learning loop for cfcd.

Implements closed-loop learning: observe OS state -> predict future ->
compare to reality -> update weights when prediction error exceeds threshold.
Includes experience buffer and OS encoder bootstrap.
"""

import time
import threading
from collections import deque
from typing import Optional

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F

from os_state_encoder import OSStateVector, OSStateEncoder
from weight_manager import WeightManager


class ExperienceBuffer:
    """Circular buffer of (state_t, state_t+1) observation pairs."""

    def __init__(self, max_size: int = 64):
        self.max_size = max_size
        self.current_states: list = []
        self.future_states: list = []
        self.position = 0
        self.full = False

    def add(self, current: torch.Tensor, future: torch.Tensor):
        """Add an observation pair."""
        if len(self.current_states) < self.max_size:
            self.current_states.append(current.detach().cpu())
            self.future_states.append(future.detach().cpu())
        else:
            self.current_states[self.position] = current.detach().cpu()
            self.future_states[self.position] = future.detach().cpu()
            self.full = True
        self.position = (self.position + 1) % self.max_size

    def sample_batch(self, batch_size: int = 16, device: str = "cpu") -> tuple:
        """Sample a random mini-batch for training."""
        n = len(self.current_states)
        indices = np.random.choice(n, size=min(batch_size, n), replace=False)
        current = torch.stack([self.current_states[i] for i in indices]).to(device)
        future = torch.stack([self.future_states[i] for i in indices]).to(device)
        return current, future

    def is_ready(self, min_samples: int = 16) -> bool:
        return len(self.current_states) >= min_samples

    def __len__(self):
        return len(self.current_states)


class OSEncoderBootstrap:
    """One-time pre-training of OSStateEncoder using temporal contrastive learning.

    Collects telemetry for a few minutes, then trains the encoder so that
    temporally adjacent states map to similar embeddings and distant states differ.
    Uses the same InfoNCE objective as the main model.
    """

    def __init__(
        self,
        encoder: OSStateEncoder,
        device: torch.device,
        collection_seconds: int = 300,
        sample_hz: int = 10,
        epochs: int = 50,
        temperature: float = 0.07,
    ):
        self.encoder = encoder
        self.device = device
        self.collection_seconds = collection_seconds
        self.sample_hz = sample_hz
        self.epochs = epochs
        self.temperature = temperature

    def collect_telemetry(self, callback=None) -> torch.Tensor:
        """Collect [N, 128] raw telemetry vectors."""
        samples = []
        interval = 1.0 / self.sample_hz
        total_samples = self.collection_seconds * self.sample_hz

        print(f"Collecting {total_samples} telemetry samples "
              f"over {self.collection_seconds}s at {self.sample_hz}Hz...")

        for i in range(total_samples):
            state = OSStateVector.collect()
            samples.append(state)

            if callback and (i + 1) % (self.sample_hz * 10) == 0:
                callback(i + 1, total_samples)

            time.sleep(interval)

        return torch.tensor(np.array(samples), dtype=torch.float32)

    def train_encoder(self, telemetry: torch.Tensor) -> dict:
        """Train encoder using temporal contrastive loss.

        Adjacent pairs (t, t+1) are positives; random pairs are negatives.
        """
        n_samples = len(telemetry)
        if n_samples < 4:
            return {"error": "Not enough samples"}

        self.encoder.train()
        self.encoder.to(self.device)
        optimizer = torch.optim.AdamW(self.encoder.parameters(), lr=1e-3, weight_decay=1e-5)

        # Create adjacent pairs
        current = telemetry[:-1].to(self.device)  # [N-1, 128]
        future = telemetry[1:].to(self.device)     # [N-1, 128]

        batch_size = min(64, len(current))
        history = {"loss": [], "accuracy": []}

        for epoch in range(self.epochs):
            # Shuffle pairs
            perm = torch.randperm(len(current))
            epoch_loss = 0.0
            epoch_acc = 0.0
            n_batches = 0

            for start in range(0, len(current) - batch_size + 1, batch_size):
                idx = perm[start : start + batch_size]
                c_batch = current[idx]
                f_batch = future[idx]

                # Encode both
                z_c = F.normalize(self.encoder(c_batch), dim=-1)
                z_f = F.normalize(self.encoder(f_batch), dim=-1)

                # InfoNCE: diagonal should be highest similarity
                logits = z_c @ z_f.T / self.temperature
                labels = torch.arange(len(logits), device=self.device)
                loss = F.cross_entropy(logits, labels)

                optimizer.zero_grad()
                loss.backward()
                torch.nn.utils.clip_grad_norm_(self.encoder.parameters(), 1.0)
                optimizer.step()

                with torch.no_grad():
                    acc = (logits.argmax(dim=1) == labels).float().mean()

                epoch_loss += loss.item()
                epoch_acc += acc.item()
                n_batches += 1

            if n_batches > 0:
                avg_loss = epoch_loss / n_batches
                avg_acc = epoch_acc / n_batches
                history["loss"].append(avg_loss)
                history["accuracy"].append(avg_acc)

                if (epoch + 1) % 10 == 0:
                    print(f"  Bootstrap epoch {epoch + 1}/{self.epochs}: "
                          f"loss={avg_loss:.4f}, acc={avg_acc*100:.1f}%")

        self.encoder.eval()
        return {
            "final_loss": history["loss"][-1] if history["loss"] else 0,
            "final_accuracy": history["accuracy"][-1] if history["accuracy"] else 0,
            "epochs": self.epochs,
            "samples": n_samples,
        }


class OnlineLearner:
    """Closed-loop online learning for the CFC-JEPA world model.

    Loop:
    1. Collect OS state at time t -> encode to embedding
    2. Predict embedding at time t+delta
    3. Wait delta milliseconds
    4. Collect OS state at time t+delta -> encode to embedding
    5. Compute prediction error (cosine distance)
    6. If error > threshold, perform gradient update
    7. Optionally version the weights
    """

    def __init__(
        self,
        model: nn.Module,
        os_encoder: OSStateEncoder,
        weight_manager: WeightManager,
        device: torch.device,
        online_lr: float = 1e-5,
        warmup_lr: float = 1e-6,
        warmup_updates: int = 100,
        grad_clip_norm: float = 1.0,
        prediction_error_threshold: float = 0.5,
        buffer_size: int = 64,
        min_buffer_for_update: int = 16,
    ):
        self.model = model
        self.os_encoder = os_encoder
        self.weight_manager = weight_manager
        self.device = device
        self.prediction_error_threshold = prediction_error_threshold
        self.grad_clip_norm = grad_clip_norm
        self.target_lr = online_lr
        self.warmup_lr = warmup_lr
        self.warmup_updates = warmup_updates
        self.min_buffer_for_update = min_buffer_for_update

        # Trainable: predictor + projection heads + OS encoder
        self.trainable_params = (
            list(model.predictor.parameters())
            + list(model.proj_predictor.parameters())
            + list(model.proj_target.parameters())
            + list(os_encoder.parameters())
        )

        self.optimizer = torch.optim.AdamW(
            self.trainable_params,
            lr=warmup_lr,
            weight_decay=1e-6,
        )

        self.buffer = ExperienceBuffer(max_size=buffer_size)

        # Statistics
        self.total_observations = 0
        self.total_updates = 0
        self.prediction_errors: deque = deque(maxlen=1000)
        self.update_history: list = []
        self.enabled = True
        self._running = False
        self._thread: Optional[threading.Thread] = None

    def _get_lr(self) -> float:
        """Get current learning rate with warmup."""
        if self.total_updates < self.warmup_updates:
            frac = self.total_updates / self.warmup_updates
            return self.warmup_lr + frac * (self.target_lr - self.warmup_lr)
        return self.target_lr

    def _update_lr(self):
        lr = self._get_lr()
        for pg in self.optimizer.param_groups:
            pg["lr"] = lr

    def encode_os_state(self) -> torch.Tensor:
        """Collect and encode current OS state to 1024-dim embedding."""
        raw = OSStateVector.collect()
        x = torch.from_numpy(raw).unsqueeze(0).to(self.device)
        with torch.no_grad():
            emb = self.os_encoder(x)
        return emb

    def predict_future(self, current_emb: torch.Tensor) -> torch.Tensor:
        """Predict future embedding from current state."""
        with torch.no_grad():
            return self.model.predict_future(current_emb, num_samples=1)

    def observe_and_learn(self, interval_sec: float = 1.0) -> dict:
        """Single observation-prediction-comparison cycle."""
        # Step 1: encode current state
        current_emb = self.encode_os_state()

        # Step 2: predict future
        predicted_future = self.predict_future(current_emb)

        # Step 3: wait
        time.sleep(interval_sec)

        # Step 4: encode actual future state
        actual_future = self.encode_os_state()

        # Step 5: compute prediction error
        with torch.no_grad():
            cos_sim = F.cosine_similarity(predicted_future, actual_future, dim=-1)
            error = 1.0 - cos_sim.item()

        self.total_observations += 1
        self.prediction_errors.append(error)
        self.weight_manager.record_prediction_error(error)

        # Step 6: add to buffer and conditionally update
        self.buffer.add(current_emb.squeeze(0), actual_future.squeeze(0))

        updated = False
        update_loss = None

        if (
            self.enabled
            and error > self.prediction_error_threshold
            and self.buffer.is_ready(self.min_buffer_for_update)
        ):
            update_loss = self._do_gradient_update()
            updated = True
            self.total_updates += 1
            self.weight_manager.record_prediction_error(error, is_post_update=True)

            # Check for auto-rollback
            rollback_version = self.weight_manager.auto_rollback_if_needed()
            if rollback_version:
                print(f"Auto-rollback to {rollback_version} (errors worsened)")

        return {
            "observation": self.total_observations,
            "prediction_error": error,
            "updated": updated,
            "update_loss": update_loss,
            "buffer_size": len(self.buffer),
            "total_updates": self.total_updates,
            "lr": self._get_lr(),
        }

    def _do_gradient_update(self) -> float:
        """Perform a single gradient update from the experience buffer."""
        self.model.train()
        self.os_encoder.train()
        self._update_lr()

        current_batch, future_batch = self.buffer.sample_batch(
            batch_size=self.min_buffer_for_update, device=str(self.device)
        )

        # Use the model's own loss function
        losses = self.model.compute_total_loss(current_batch, future_batch)
        total_loss = losses["total_loss"]

        self.optimizer.zero_grad()
        total_loss.backward()
        torch.nn.utils.clip_grad_norm_(self.trainable_params, self.grad_clip_norm)
        self.optimizer.step()

        self.model.eval()
        self.os_encoder.eval()

        self.update_history.append({
            "update_num": self.total_updates + 1,
            "loss": total_loss.item(),
            "diffusion_loss": losses["diffusion_loss"].item(),
            "infonce_loss": losses["infonce_loss"].item(),
            "lr": self._get_lr(),
            "timestamp": time.time(),
        })

        return total_loss.item()

    def run_loop(self, interval_sec: float = 1.0, max_iterations: int = 0):
        """Main observation-prediction-comparison loop. Runs in current thread."""
        self._running = True
        iteration = 0

        print(f"Online learning loop started (interval={interval_sec}s, "
              f"threshold={self.prediction_error_threshold})")

        while self._running:
            result = self.observe_and_learn(interval_sec)

            if result["updated"]:
                print(f"  [Update {result['total_updates']}] "
                      f"error={result['prediction_error']:.4f} "
                      f"loss={result['update_loss']:.4f} "
                      f"lr={result['lr']:.2e}")

            iteration += 1
            if max_iterations > 0 and iteration >= max_iterations:
                break

        self._running = False

    def start_background(self, interval_sec: float = 1.0):
        """Start the learning loop in a background thread."""
        if self._thread and self._thread.is_alive():
            return
        self._running = True
        self._thread = threading.Thread(
            target=self.run_loop, args=(interval_sec,), daemon=True
        )
        self._thread.start()

    def stop(self):
        """Stop the background learning loop."""
        self._running = False
        if self._thread:
            self._thread.join(timeout=5)

    def get_stats(self) -> dict:
        errors = list(self.prediction_errors)
        return {
            "total_observations": self.total_observations,
            "total_updates": self.total_updates,
            "update_rate": self.total_updates / max(self.total_observations, 1),
            "mean_prediction_error": sum(errors) / max(len(errors), 1),
            "recent_errors": errors[-10:],
            "buffer_fill": len(self.buffer),
            "current_lr": self._get_lr(),
            "enabled": self.enabled,
        }
