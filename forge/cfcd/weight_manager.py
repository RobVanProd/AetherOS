"""
Weight versioning and hot-reload for cfcd.

Manages versioned model checkpoints with atomic saves, manifest tracking,
symlink-based current version, and auto-rollback on prediction degradation.
"""

import json
import os
import tempfile
import time
from collections import deque
from pathlib import Path
from typing import Optional

import torch
import torch.nn as nn


class WeightManager:
    """Manages versioned model weights with rollback capability."""

    def __init__(
        self,
        model: nn.Module,
        base_dir: str = "/var/lib/aether/aurora/models",
        max_versions: int = 10,
        auto_rollback_window: int = 100,
    ):
        self.model = model
        self.base_dir = Path(base_dir)
        self.base_dir.mkdir(parents=True, exist_ok=True)
        self.max_versions = max_versions
        self.current_version = 0
        self.manifest_path = self.base_dir / "manifest.json"

        # Auto-rollback tracking
        self.auto_rollback_window = auto_rollback_window
        self.pre_update_errors: deque = deque(maxlen=auto_rollback_window)
        self.post_update_errors: deque = deque(maxlen=auto_rollback_window)
        self.last_good_version: Optional[str] = None

        # Load existing manifest
        self.manifest = self._load_manifest()
        if self.manifest:
            self.current_version = max(e["version_num"] for e in self.manifest)

    def _load_manifest(self) -> list:
        if self.manifest_path.exists():
            with open(self.manifest_path) as f:
                return json.load(f)
        return []

    def _save_manifest(self):
        with open(self.manifest_path, "w") as f:
            json.dump(self.manifest, f, indent=2)

    def save_version(self, metrics: Optional[dict] = None) -> str:
        """Save current weights as a new version. Returns version string."""
        self.current_version += 1
        version_str = f"v{self.current_version:04d}"
        timestamp = time.strftime("%Y%m%d_%H%M%S")
        filename = f"{version_str}_{timestamp}.pt"
        filepath = self.base_dir / filename

        # Atomic save: write to temp then rename
        state = {
            "model_state_dict": self.model.state_dict(),
            "version": version_str,
            "timestamp": timestamp,
            "metrics": metrics or {},
        }
        fd, tmp_path = tempfile.mkstemp(dir=str(self.base_dir), suffix=".pt.tmp")
        os.close(fd)
        try:
            torch.save(state, tmp_path)
            os.rename(tmp_path, str(filepath))
        except Exception:
            if os.path.exists(tmp_path):
                os.unlink(tmp_path)
            raise

        # Update current symlink
        current_link = self.base_dir / "current"
        tmp_link = self.base_dir / "current.tmp"
        try:
            os.symlink(str(filepath), str(tmp_link))
            os.rename(str(tmp_link), str(current_link))
        except OSError:
            # Fallback: remove and recreate
            if current_link.exists() or current_link.is_symlink():
                current_link.unlink()
            os.symlink(str(filepath), str(current_link))

        # Update manifest
        entry = {
            "version": version_str,
            "version_num": self.current_version,
            "filename": filename,
            "path": str(filepath),
            "timestamp": timestamp,
            "metrics": metrics or {},
        }
        self.manifest.append(entry)
        self._save_manifest()

        self.last_good_version = version_str

        # Prune old versions
        self._prune_old_versions()

        return version_str

    def rollback(self, version: str) -> bool:
        """Rollback to a specific version. Returns True on success."""
        entry = next((e for e in self.manifest if e["version"] == version), None)
        if entry is None:
            return False

        path = Path(entry["path"])
        if not path.exists():
            return False

        return self.hot_reload(str(path))

    def hot_reload(self, checkpoint_path: str) -> bool:
        """Load weights from a checkpoint into the running model."""
        try:
            ckpt = torch.load(checkpoint_path, map_location="cpu", weights_only=False)
            state_dict = ckpt.get("model_state_dict", ckpt)

            # Filter out gate_values buffers (may have shape mismatches)
            filtered = {k: v for k, v in state_dict.items() if "gate_values" not in k}
            self.model.load_state_dict(filtered, strict=False)
            return True
        except Exception as e:
            print(f"Hot reload failed: {e}")
            return False

    def record_prediction_error(self, error: float, is_post_update: bool = False):
        """Record prediction error for auto-rollback tracking."""
        if is_post_update:
            self.post_update_errors.append(error)
        else:
            self.pre_update_errors.append(error)

    def should_rollback(self) -> bool:
        """Check if prediction errors have worsened since last update."""
        if (
            len(self.pre_update_errors) < self.auto_rollback_window // 2
            or len(self.post_update_errors) < self.auto_rollback_window // 2
        ):
            return False

        pre_mean = sum(self.pre_update_errors) / len(self.pre_update_errors)
        post_mean = sum(self.post_update_errors) / len(self.post_update_errors)

        # Rollback if post-update error is 20% worse
        return post_mean > pre_mean * 1.2

    def auto_rollback_if_needed(self) -> Optional[str]:
        """Check and perform auto-rollback if errors have worsened."""
        if self.should_rollback() and self.last_good_version:
            if self.rollback(self.last_good_version):
                self.post_update_errors.clear()
                return self.last_good_version
        return None

    def get_manifest(self) -> list:
        return self.manifest

    def get_current_version(self) -> str:
        if self.manifest:
            return self.manifest[-1]["version"]
        return "v0000"

    def _prune_old_versions(self):
        """Delete versions beyond max_versions, keeping current."""
        if len(self.manifest) <= self.max_versions:
            return

        # Keep the most recent max_versions entries
        to_remove = self.manifest[: -self.max_versions]
        self.manifest = self.manifest[-self.max_versions :]

        for entry in to_remove:
            path = Path(entry["path"])
            if path.exists():
                path.unlink()

        self._save_manifest()
