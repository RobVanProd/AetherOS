#!/usr/bin/env python3
"""
cfcd — CFC-JEPA Model Runtime Daemon for AetherOS.

Loads the trained CFC-JEPA world model, serves inference over a Unix socket,
supports online learning with weight versioning and OS state encoding.

Usage:
    python cfcd_server.py --checkpoint /path/to/model_final.pt
    python cfcd_server.py --checkpoint /path/to/model_final.pt --enable-learning
"""

import argparse
import json
import os
import socket
import sys
import time
import traceback
from pathlib import Path

import torch

# Add model source to path
MODEL_SRC = "/home/rob/jepaworlddiffusionlm/internal_world_model"
sys.path.insert(0, MODEL_SRC)

from models.cfc_jepa_world_model import CFCJEPAWorldModel, CFCJEPAConfig

from config import CFCDConfig
from os_state_encoder import OSStateEncoder, OSStateVector
from weight_manager import WeightManager
from online_learner import OnlineLearner, OSEncoderBootstrap


class CFCDaemon:
    """Model runtime daemon. Loads CFC-JEPA, serves inference over Unix socket."""

    def __init__(self, config: CFCDConfig):
        self.config = config
        self.start_time = time.time()

        # Device
        if config.device == "auto":
            self.device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        else:
            self.device = torch.device(config.device)
        print(f"Device: {self.device}")

        # Load model
        self.model, self.model_config = self._load_model(config.checkpoint_path)
        self.model.eval()

        param_count = sum(p.numel() for p in self.model.parameters())
        print(f"Model parameters: {param_count:,}")

        # OS State Encoder
        self.os_encoder = OSStateEncoder(
            input_dim=config.os_feature_dim,
            output_dim=config.encoder_output_dim,
        ).to(self.device)
        self.os_encoder.eval()

        # Weight Manager
        weight_dir = config.resolve_weight_dir()
        self.weight_manager = WeightManager(
            self.model, str(weight_dir), config.max_weight_versions,
            config.auto_rollback_window,
        )

        # Online Learner
        self.learner = OnlineLearner(
            model=self.model,
            os_encoder=self.os_encoder,
            weight_manager=self.weight_manager,
            device=self.device,
            online_lr=config.online_lr,
            warmup_lr=config.warmup_lr,
            warmup_updates=config.warmup_updates,
            grad_clip_norm=config.grad_clip_norm,
            prediction_error_threshold=config.prediction_error_threshold,
            buffer_size=config.telemetry_buffer_size,
            min_buffer_for_update=config.min_buffer_for_update,
        )
        self.learner.enabled = config.online_learning_enabled

        # Prediction tracking
        self.total_predictions = 0
        self.total_latency_ms = 0.0

    def _load_model(self, checkpoint_path: str):
        """Load CFC-JEPA model from checkpoint."""
        print(f"Loading checkpoint: {checkpoint_path}")

        ckpt = torch.load(checkpoint_path, map_location=self.device, weights_only=False)

        # Reconstruct config from training_summary.json
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

        print(f"Config: encoder_dim={config.encoder_dim}, "
              f"hidden_dim={config.hidden_dim}, "
              f"diffusion_steps={config.diffusion_steps}, "
              f"use_cfc={config.use_cfc}")

        model = CFCJEPAWorldModel(config).to(self.device)

        # Load weights
        state_dict = ckpt.get("model_state_dict", ckpt)
        filtered = {k: v for k, v in state_dict.items() if "gate_values" not in k}
        missing, unexpected = model.load_state_dict(filtered, strict=False)
        if missing:
            print(f"  Missing keys (expected for buffers): {len(missing)}")

        return model, config

    # --- HTTP Request Handling ---

    def handle_request(self, method: str, path: str, body: str) -> tuple:
        """Route request to handler. Returns (status_code, response_dict)."""
        try:
            if method == "GET" and path == "/v0/health":
                return 200, self._handle_health()
            elif method == "GET" and path == "/v0/introspect":
                return 200, self._handle_introspect()
            elif method == "POST" and path == "/v0/predict":
                return 200, self._handle_predict(json.loads(body) if body else {})
            elif method == "POST" and path == "/v0/encode_state":
                return 200, self._handle_encode_state(json.loads(body) if body else {})
            elif method == "POST" and path == "/v0/update_weights":
                return 200, self._handle_update_weights(json.loads(body) if body else {})
            elif method == "POST" and path == "/v0/learning/enable":
                self.learner.enabled = True
                return 200, {"ok": True, "learning_enabled": True}
            elif method == "POST" and path == "/v0/learning/disable":
                self.learner.enabled = False
                return 200, {"ok": True, "learning_enabled": False}
            elif method == "POST" and path == "/v0/weights/save":
                version = self.weight_manager.save_version()
                return 200, {"ok": True, "version": version}
            elif method == "POST" and path == "/v0/weights/rollback":
                req = json.loads(body) if body else {}
                version = req.get("version", "")
                ok = self.weight_manager.rollback(version)
                return 200, {"ok": ok, "rolled_back_to": version}
            else:
                return 404, {"error": f"Not found: {method} {path}"}
        except Exception as e:
            traceback.print_exc()
            return 500, {"error": str(e)}

    def _handle_health(self) -> dict:
        return {
            "ok": True,
            "service": "cfcd",
            "version": "0.1.0",
            "device": str(self.device),
            "param_count": sum(p.numel() for p in self.model.parameters()),
            "weight_version": self.weight_manager.get_current_version(),
            "uptime_seconds": int(time.time() - self.start_time),
            "learning_enabled": self.learner.enabled,
            "total_predictions": self.total_predictions,
        }

    def _handle_predict(self, request: dict) -> dict:
        state_list = request.get("state")
        if state_list is None:
            # If no state provided, encode current OS state
            emb = self.learner.encode_os_state()
        else:
            emb = torch.tensor([state_list], dtype=torch.float32, device=self.device)

        num_samples = request.get("num_samples", 1)

        t0 = time.time()
        with torch.no_grad():
            predicted = self.model.predict_future(emb, num_samples=num_samples)
        latency_ms = (time.time() - t0) * 1000

        self.total_predictions += 1
        self.total_latency_ms += latency_ms

        # Gate stats
        gate_stats = {}
        try:
            raw_stats = self.model.get_gate_stats()
            for k, v in raw_stats.items():
                gate_stats[str(k)] = {
                    "mean": float(v[0]),
                    "std": float(v[1]),
                    "min": float(v[2]),
                    "max": float(v[3]),
                }
        except Exception:
            pass

        return {
            "ok": True,
            "prediction": predicted.squeeze(0).cpu().tolist(),
            "gate_stats": gate_stats,
            "latency_ms": round(latency_ms, 2),
            "num_samples": num_samples,
        }

    def _handle_encode_state(self, request: dict) -> dict:
        """Encode OS telemetry to 1024-dim embedding."""
        if "telemetry" in request:
            raw = torch.tensor([request["telemetry"]], dtype=torch.float32, device=self.device)
        else:
            raw_np = OSStateVector.collect()
            raw = torch.from_numpy(raw_np).unsqueeze(0).to(self.device)

        with torch.no_grad():
            emb = self.os_encoder(raw)

        return {
            "ok": True,
            "embedding": emb.squeeze(0).cpu().tolist(),
            "input_dim": raw.shape[-1],
            "output_dim": emb.shape[-1],
        }

    def _handle_update_weights(self, request: dict) -> dict:
        """Trigger a single online learning observation cycle."""
        if not self.learner.enabled:
            return {"ok": False, "error": "Online learning is disabled"}

        result = self.learner.observe_and_learn(
            interval_sec=request.get("interval_sec", 1.0)
        )

        # Save version if weights were updated
        if result["updated"]:
            version = self.weight_manager.save_version(
                metrics={"prediction_error": result["prediction_error"],
                         "update_loss": result["update_loss"]}
            )
            result["weight_version"] = version

        return {"ok": True, **result}

    def _handle_introspect(self) -> dict:
        """Full model introspection."""
        gate_stats = {}
        try:
            raw_stats = self.model.get_gate_stats()
            for k, v in raw_stats.items():
                gate_stats[str(k)] = {
                    "mean": float(v[0]), "std": float(v[1]),
                    "min": float(v[2]), "max": float(v[3]),
                }
        except Exception:
            pass

        avg_latency = (self.total_latency_ms / self.total_predictions
                       if self.total_predictions > 0 else 0)

        return {
            "model": {
                "weight_version": self.weight_manager.get_current_version(),
                "param_count": sum(p.numel() for p in self.model.parameters()),
                "encoder_dim": self.model_config.encoder_dim,
                "hidden_dim": self.model_config.hidden_dim,
                "diffusion_steps": self.model_config.diffusion_steps,
                "use_cfc": self.model_config.use_cfc,
                "device": str(self.device),
                "uptime_seconds": int(time.time() - self.start_time),
            },
            "gates": gate_stats,
            "predictions": {
                "total_predictions": self.total_predictions,
                "mean_latency_ms": round(avg_latency, 2),
            },
            "learning": self.learner.get_stats(),
            "versions": self.weight_manager.get_manifest()[-5:],
        }

    # --- Unix Socket HTTP Server ---

    def run(self):
        """Start the Unix socket HTTP server."""
        sock_path = self.config.socket_path

        # Remove stale socket
        if os.path.exists(sock_path):
            os.unlink(sock_path)

        server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        server.bind(sock_path)
        server.listen(5)
        os.chmod(sock_path, 0o666)

        print(f"\ncfcd listening on {sock_path}")
        print(f"  Health: curl --unix-socket {sock_path} http://localhost/v0/health")
        print(f"  Predict: curl --unix-socket {sock_path} -X POST "
              f"-d '{{}}' http://localhost/v0/predict\n")

        # Start background learning if enabled
        if self.config.online_learning_enabled:
            interval_sec = self.config.telemetry_interval_ms / 1000.0
            self.learner.start_background(interval_sec)
            print(f"Online learning started (interval={interval_sec}s)")

        try:
            while True:
                conn, _ = server.accept()
                try:
                    self._handle_connection(conn)
                except Exception as e:
                    print(f"Connection error: {e}")
                finally:
                    conn.close()
        except KeyboardInterrupt:
            print("\nShutting down cfcd...")
        finally:
            self.learner.stop()
            server.close()
            if os.path.exists(sock_path):
                os.unlink(sock_path)

    def _handle_connection(self, conn: socket.socket):
        """Handle a single HTTP connection."""
        data = b""
        while True:
            chunk = conn.recv(4096)
            if not chunk:
                break
            data += chunk
            if b"\r\n\r\n" in data:
                break

        if not data:
            return

        request_str = data.decode("utf-8", errors="replace")
        header_end = request_str.index("\r\n\r\n")
        headers = request_str[:header_end]
        body = request_str[header_end + 4:]

        # Parse request line
        first_line = headers.split("\r\n")[0]
        parts = first_line.split(" ")
        method = parts[0] if len(parts) >= 1 else "GET"
        path = parts[1] if len(parts) >= 2 else "/"

        # Check Content-Length for body
        content_length = 0
        for line in headers.split("\r\n"):
            if line.lower().startswith("content-length:"):
                content_length = int(line.split(":")[1].strip())

        # Read remaining body if needed
        while len(body.encode()) < content_length:
            chunk = conn.recv(4096)
            if not chunk:
                break
            body += chunk.decode("utf-8", errors="replace")

        # Handle request
        status_code, response = self.handle_request(method, path, body)

        # Send HTTP response
        response_json = json.dumps(response)
        http_response = (
            f"HTTP/1.1 {status_code} OK\r\n"
            f"Content-Type: application/json\r\n"
            f"Content-Length: {len(response_json)}\r\n"
            f"\r\n"
            f"{response_json}"
        )
        conn.sendall(http_response.encode())


def main():
    parser = argparse.ArgumentParser(description="cfcd - CFC-JEPA Model Runtime")
    parser.add_argument("--checkpoint", type=str, required=True,
                        help="Path to model checkpoint (.pt)")
    parser.add_argument("--socket", type=str, default="/tmp/cfcd.sock",
                        help="Unix socket path")
    parser.add_argument("--device", type=str, default="auto")
    parser.add_argument("--enable-learning", action="store_true",
                        help="Enable online learning on startup")
    parser.add_argument("--learning-interval", type=float, default=1.0,
                        help="Seconds between learning observations")
    parser.add_argument("--bootstrap", action="store_true",
                        help="Bootstrap OS encoder before starting server")
    parser.add_argument("--bootstrap-seconds", type=int, default=60,
                        help="Seconds of telemetry to collect for bootstrap")
    args = parser.parse_args()

    config = CFCDConfig(
        checkpoint_path=args.checkpoint,
        socket_path=args.socket,
        device=args.device,
        online_learning_enabled=args.enable_learning,
        telemetry_interval_ms=int(args.learning_interval * 1000),
        bootstrap_duration_sec=args.bootstrap_seconds,
    )

    daemon = CFCDaemon(config)

    # Optional: bootstrap OS encoder
    if args.bootstrap:
        print("\n--- OS Encoder Bootstrap ---")
        bootstrap = OSEncoderBootstrap(
            encoder=daemon.os_encoder,
            device=daemon.device,
            collection_seconds=config.bootstrap_duration_sec,
            sample_hz=config.bootstrap_sample_hz,
            epochs=config.bootstrap_epochs,
        )
        telemetry = bootstrap.collect_telemetry(
            callback=lambda i, n: print(f"  Collected {i}/{n} samples")
        )
        result = bootstrap.train_encoder(telemetry)
        print(f"Bootstrap complete: acc={result.get('final_accuracy', 0)*100:.1f}%")

    daemon.run()


if __name__ == "__main__":
    main()
