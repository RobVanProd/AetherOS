"""
OS State Encoder for cfcd.

Collects system telemetry (CPU, memory, GPU, processes) into a fixed-size
feature vector and projects it into the CFC-JEPA 1024-dim embedding space
via a learned MLP.
"""

import numpy as np
import subprocess
import torch
import torch.nn as nn

try:
    import psutil
except ImportError:
    psutil = None


class OSStateVector:
    """Collects raw OS telemetry into a normalized 128-dim feature vector."""

    FEATURE_DIM = 128

    @staticmethod
    def collect() -> np.ndarray:
        """Collect current OS state. Returns [128] float32 vector, values in [0, 1]."""
        features = np.zeros(OSStateVector.FEATURE_DIM, dtype=np.float32)
        idx = 0

        if psutil is None:
            return features

        # CPU per-core utilization (16 dims, capped at 16 cores)
        cpu_pcts = psutil.cpu_percent(percpu=True)
        for i in range(min(16, len(cpu_pcts))):
            features[idx + i] = cpu_pcts[i] / 100.0
        idx += 16

        # CPU frequencies per core (16 dims, normalized by max freq)
        try:
            freqs = psutil.cpu_freq(percpu=True)
            if freqs:
                max_freq = max(f.max for f in freqs) if freqs[0].max > 0 else 6000.0
                for i in range(min(16, len(freqs))):
                    features[idx + i] = freqs[i].current / max_freq
        except Exception:
            pass
        idx += 16

        # Memory stats (4 dims)
        mem = psutil.virtual_memory()
        features[idx] = mem.total / (256 * 1024**3)  # normalize to 256GB
        features[idx + 1] = mem.available / mem.total
        features[idx + 2] = mem.percent / 100.0
        swap = psutil.swap_memory()
        features[idx + 3] = swap.percent / 100.0 if swap.total > 0 else 0.0
        idx += 4

        # Disk I/O counters (4 dims)
        try:
            dio = psutil.disk_io_counters()
            if dio:
                features[idx] = min(dio.read_bytes / (1024**3), 1.0)
                features[idx + 1] = min(dio.write_bytes / (1024**3), 1.0)
                features[idx + 2] = min(dio.read_count / 1e6, 1.0)
                features[idx + 3] = min(dio.write_count / 1e6, 1.0)
        except Exception:
            pass
        idx += 4

        # Network I/O (8 dims - 2 per top interface, max 4 interfaces)
        try:
            net = psutil.net_io_counters(pernic=True)
            ifaces = sorted(net.keys())[:4]
            for i, iface in enumerate(ifaces):
                c = net[iface]
                features[idx + i * 2] = min(c.bytes_sent / (1024**3), 1.0)
                features[idx + i * 2 + 1] = min(c.bytes_recv / (1024**3), 1.0)
        except Exception:
            pass
        idx += 8

        # Process stats (4 dims)
        try:
            statuses = [p.status() for p in psutil.process_iter(["status"])]
            total = len(statuses)
            features[idx] = min(total / 1000.0, 1.0)
            features[idx + 1] = sum(1 for s in statuses if s == "running") / max(total, 1)
            features[idx + 2] = sum(1 for s in statuses if s == "sleeping") / max(total, 1)
            features[idx + 3] = sum(1 for s in statuses if s == "zombie") / max(total, 1)
        except Exception:
            pass
        idx += 4

        # Load averages (3 dims)
        try:
            load = psutil.getloadavg()
            n_cpus = psutil.cpu_count() or 16
            for i in range(3):
                features[idx + i] = min(load[i] / n_cpus, 1.0)
        except Exception:
            pass
        idx += 3

        # GPU via rocm-smi (4 dims)
        try:
            result = subprocess.run(
                ["rocm-smi", "--showuse", "--showmeminfo", "vram", "--showtemp", "--csv"],
                capture_output=True, text=True, timeout=2,
            )
            if result.returncode == 0:
                lines = result.stdout.strip().split("\n")
                if len(lines) >= 2:
                    # Parse CSV-like output
                    parts = lines[1].split(",")
                    if len(parts) >= 2:
                        features[idx] = float(parts[1].strip().rstrip("%")) / 100.0  # GPU use
        except Exception:
            pass
        idx += 4

        # Top 10 processes by CPU (20 dims: cpu%, mem% each)
        try:
            procs = sorted(
                psutil.process_iter(["cpu_percent", "memory_percent"]),
                key=lambda p: p.info.get("cpu_percent", 0) or 0,
                reverse=True,
            )[:10]
            for i, p in enumerate(procs):
                features[idx + i * 2] = min((p.info.get("cpu_percent", 0) or 0) / 100.0, 1.0)
                features[idx + i * 2 + 1] = min((p.info.get("memory_percent", 0) or 0) / 100.0, 1.0)
        except Exception:
            pass
        idx += 20

        # Top 10 processes by memory (20 dims)
        try:
            procs = sorted(
                psutil.process_iter(["cpu_percent", "memory_percent"]),
                key=lambda p: p.info.get("memory_percent", 0) or 0,
                reverse=True,
            )[:10]
            for i, p in enumerate(procs):
                features[idx + i * 2] = min((p.info.get("cpu_percent", 0) or 0) / 100.0, 1.0)
                features[idx + i * 2 + 1] = min((p.info.get("memory_percent", 0) or 0) / 100.0, 1.0)
        except Exception:
            pass
        idx += 20

        # System counters (5 dims)
        try:
            ctx = psutil.cpu_stats()
            features[idx] = min(ctx.ctx_switches / 1e9, 1.0)
            features[idx + 1] = min(ctx.interrupts / 1e9, 1.0)
            features[idx + 2] = min(ctx.syscalls / 1e9, 1.0) if hasattr(ctx, "syscalls") else 0.0
        except Exception:
            pass
        try:
            import time
            features[idx + 3] = min(time.time() / 2e9, 1.0)  # epoch time normalized
            features[idx + 4] = min(psutil.boot_time() / 2e9, 1.0)
        except Exception:
            pass
        idx += 5

        # Remaining dims are padding (zeros)
        return np.clip(features, 0.0, 1.0)


class OSStateEncoder(nn.Module):
    """Learned projection from OS telemetry to CFC-JEPA embedding space.

    Architecture: 128 -> 512 -> 1024 -> 1024 with LayerNorm and GELU.
    ~1.3M parameters, small enough to train online.
    """

    def __init__(self, input_dim: int = 128, output_dim: int = 1024):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(input_dim, 512),
            nn.LayerNorm(512),
            nn.GELU(),
            nn.Linear(512, 1024),
            nn.LayerNorm(1024),
            nn.GELU(),
            nn.Linear(1024, output_dim),
            nn.LayerNorm(output_dim),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """[B, 128] -> [B, 1024]"""
        return self.net(x)


if __name__ == "__main__":
    print("OS State Encoder - Telemetry Test")
    print("=" * 50)
    state = OSStateVector.collect()
    print(f"Feature vector shape: {state.shape}")
    print(f"Non-zero features: {np.count_nonzero(state)}/{len(state)}")
    print(f"Value range: [{state.min():.4f}, {state.max():.4f}]")

    encoder = OSStateEncoder()
    n_params = sum(p.numel() for p in encoder.parameters())
    print(f"\nEncoder parameters: {n_params:,}")

    x = torch.from_numpy(state).unsqueeze(0)
    emb = encoder(x)
    print(f"Embedding shape: {emb.shape}")
    print(f"Embedding stats: mean={emb.mean():.4f}, std={emb.std():.4f}")
