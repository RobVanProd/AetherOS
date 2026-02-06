"""Configuration for the cfcd model runtime daemon."""

from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class CFCDConfig:
    # Model loading
    checkpoint_path: str = ""
    model_source_dir: str = "/home/rob/jepaworlddiffusionlm/internal_world_model"

    # Server
    socket_path: str = "/tmp/cfcd.sock"

    # Device
    device: str = "auto"  # "auto", "cuda", "cpu"

    # Inference
    ddim_steps: int = 10
    max_batch_size: int = 16

    # Online learning
    online_learning_enabled: bool = False
    online_lr: float = 1e-5
    online_weight_decay: float = 1e-6
    grad_clip_norm: float = 1.0
    prediction_error_threshold: float = 0.5
    telemetry_buffer_size: int = 64
    telemetry_interval_ms: int = 1000
    min_buffer_for_update: int = 16
    warmup_updates: int = 100
    warmup_lr: float = 1e-6

    # Weight versioning
    weight_version_dir: str = "/var/lib/aether/aurora/models"
    max_weight_versions: int = 10
    auto_rollback_window: int = 100

    # OS State Encoder
    os_feature_dim: int = 128
    encoder_output_dim: int = 1024
    bootstrap_duration_sec: int = 300
    bootstrap_sample_hz: int = 10
    bootstrap_epochs: int = 50

    def resolve_weight_dir(self) -> Path:
        """Return weight dir, creating if needed. Falls back to local."""
        p = Path(self.weight_version_dir)
        try:
            p.mkdir(parents=True, exist_ok=True)
            return p
        except PermissionError:
            fallback = Path.home() / ".aether" / "aurora" / "models"
            fallback.mkdir(parents=True, exist_ok=True)
            return fallback
