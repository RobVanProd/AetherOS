#!/usr/bin/env python3
"""
CFC-JEPA World Model
Aeternum Labs | January 2026

JEPA-style world model with CFC-Diffusion predictor.
Architecture follows VL-JEPA (Chen et al., Dec 2025):
- Frozen state encoder (V-JEPA 2 or DINOv2)
- CFC-Diffusion predictor (trainable, 17M validated)
- InfoNCE loss in embedding space

Key innovations:
- Diffusion generates diverse futures
- CFC provides temporal coherence
- Predicting embeddings, not pixels
"""

import torch
import torch.nn as nn
import torch.nn.functional as F
import math
from typing import Optional, Tuple, Dict
import sys
import os

sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from models.diffusion_predictor import DiffusionPredictor


class CFCJEPAConfig:
    """Configuration for CFC-JEPA World Model."""
    
    def __init__(
        self,
        encoder_dim: int = 1024,      # V-JEPA 2 ViT-L output dim
        hidden_dim: int = 512,         # CFC predictor hidden (validated)
        embed_dim: int = 1536,         # Projection space (per VL-JEPA)
        num_layers: int = 4,           # CFC layers (validated)
        num_heads: int = 8,
        diffusion_steps: int = 100,
        temperature: float = 0.07,     # InfoNCE temperature
        use_cfc: bool = True,
    ):
        self.encoder_dim = encoder_dim
        self.hidden_dim = hidden_dim
        self.embed_dim = embed_dim
        self.num_layers = num_layers
        self.num_heads = num_heads
        self.diffusion_steps = diffusion_steps
        self.temperature = temperature
        self.use_cfc = use_cfc


class ProjectionHead(nn.Module):
    """MLP projection head for embedding space."""
    
    def __init__(self, in_dim: int, out_dim: int, hidden_dim: int = 2048):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(in_dim, hidden_dim),
            nn.GELU(),
            nn.Linear(hidden_dim, hidden_dim),
            nn.GELU(),
            nn.Linear(hidden_dim, out_dim),
        )
    
    def forward(self, x):
        return self.net(x)


class CFCJEPAWorldModel(nn.Module):
    """
    JEPA-style world model with CFC-Diffusion predictor.
    
    Predicts future state embeddings from current state embeddings.
    
    Architecture (following VL-JEPA):
    - X-Encoder: Frozen state encoder for current observation
    - Predictor: CFC-Diffusion blocks (trainable)
    - Y-Encoder: Same frozen encoder for target (or EMA)
    - Loss: InfoNCE in projected embedding space
    """
    
    def __init__(self, config: CFCJEPAConfig, state_encoder: Optional[nn.Module] = None):
        super().__init__()
        self.config = config
        
        # State encoder (frozen)
        if state_encoder is not None:
            self.state_encoder = state_encoder
            for p in self.state_encoder.parameters():
                p.requires_grad = False
        else:
            # Placeholder: identity when working with pre-computed embeddings
            self.state_encoder = None
        
        # CFC-Diffusion Predictor (trainable)
        self.predictor = DiffusionPredictor(
            state_dim=config.encoder_dim,
            hidden_dim=config.hidden_dim,
            num_layers=config.num_layers,
            num_heads=config.num_heads,
            num_timesteps=config.diffusion_steps,
            use_cfc=config.use_cfc,
        )
        
        # Projection heads (trainable)
        self.proj_predictor = ProjectionHead(
            config.encoder_dim, config.embed_dim
        )
        self.proj_target = ProjectionHead(
            config.encoder_dim, config.embed_dim
        )
        
        # Temperature for InfoNCE
        self.temperature = config.temperature
        
    def encode(self, x: torch.Tensor) -> torch.Tensor:
        """Encode observations to state embeddings (frozen)."""
        if self.state_encoder is not None:
            with torch.no_grad():
                return self.state_encoder(x)
        else:
            # x is already embeddings
            return x
    
    def predict_future(
        self, 
        current_state: torch.Tensor,
        num_samples: int = 1
    ) -> torch.Tensor:
        """
        Predict future state embedding via CFC-Diffusion.
        
        Args:
            current_state: [B, D] current state embedding
            num_samples: Number of future predictions per input
            
        Returns:
            future_state: [B, D] predicted future embedding
        """
        return self.predictor.sample(current_state, num_samples=num_samples)
    
    def forward(
        self,
        current_emb: torch.Tensor,
        future_emb: torch.Tensor,
        timestep: Optional[torch.Tensor] = None
    ) -> Tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        """
        Training forward pass.
        
        Args:
            current_emb: [B, D] current state embeddings
            future_emb: [B, D] target future embeddings
            timestep: Diffusion timestep (random if None)
            
        Returns:
            z_pred: [B, embed_dim] projected predicted embedding
            z_target: [B, embed_dim] projected target embedding
            diff_loss: Diffusion denoising loss
        """
        B = current_emb.size(0)
        device = current_emb.device
        
        # Random timestep if not provided
        if timestep is None:
            timestep = torch.randint(
                0, self.config.diffusion_steps, (B,), device=device
            )
        
        # Diffusion training: predict noise
        pred_noise, true_noise = self.predictor(
            future_emb, current_emb, timestep
        )
        diff_loss = F.mse_loss(pred_noise, true_noise)
        
        # For InfoNCE: sample clean predictions
        with torch.no_grad():
            future_pred = self.predict_future(current_emb, num_samples=1)
        
        # Project to embedding space
        z_pred = self.proj_predictor(future_pred)
        z_target = self.proj_target(future_emb)
        
        return z_pred, z_target, diff_loss
    
    def compute_infonce_loss(
        self,
        z_pred: torch.Tensor,
        z_target: torch.Tensor
    ) -> Dict[str, torch.Tensor]:
        """
        InfoNCE loss following VL-JEPA.
        
        Combines:
        - Alignment: predicted should match target
        - Uniformity: embeddings should be spread out (anti-collapse)
        """
        # Normalize embeddings
        z_pred = F.normalize(z_pred, dim=-1)
        z_target = F.normalize(z_target, dim=-1)
        
        # Cosine similarity matrix
        logits = z_pred @ z_target.T / self.temperature
        
        # InfoNCE: diagonal elements should be highest
        B = z_pred.size(0)
        labels = torch.arange(B, device=z_pred.device)
        
        # Cross-entropy loss (both directions for symmetry)
        loss_pred = F.cross_entropy(logits, labels)
        loss_target = F.cross_entropy(logits.T, labels)
        infonce_loss = (loss_pred + loss_target) / 2
        
        # Metrics
        with torch.no_grad():
            # Accuracy: how often diagonal is max
            pred_correct = (logits.argmax(dim=1) == labels).float().mean()
            
            # Uniformity: -log of average pairwise similarity (lower = more uniform)
            sq_pdist = torch.pdist(z_pred, p=2).pow(2)
            uniformity = sq_pdist.mul(-2).exp().mean().log()
        
        return {
            "infonce_loss": infonce_loss,
            "accuracy": pred_correct,
            "uniformity": uniformity,
        }
    
    def compute_total_loss(
        self,
        current_emb: torch.Tensor,
        future_emb: torch.Tensor,
        infonce_weight: float = 1.0,
        diffusion_weight: float = 1.0
    ) -> Dict[str, torch.Tensor]:
        """
        Compute total training loss.
        
        Combines:
        - Diffusion denoising loss (learn to predict noise)
        - InfoNCE loss (align embeddings, prevent collapse)
        """
        z_pred, z_target, diff_loss = self.forward(current_emb, future_emb)
        infonce_metrics = self.compute_infonce_loss(z_pred, z_target)
        
        total_loss = (
            diffusion_weight * diff_loss + 
            infonce_weight * infonce_metrics["infonce_loss"]
        )
        
        return {
            "total_loss": total_loss,
            "diffusion_loss": diff_loss,
            **infonce_metrics
        }
    
    def get_gate_stats(self) -> Dict:
        """Get CFC gate statistics from predictor."""
        return self.predictor.get_gate_stats()


# =============================================================================
# ENCODER LOADING UTILITIES
# =============================================================================

def load_vjepa2_encoder(
    weights_path: Optional[str] = None,
    model_type: str = "vit-l"
) -> nn.Module:
    """
    Load V-JEPA 2 encoder (frozen).
    
    Note: Requires V-JEPA 2 weights from Meta.
    Download from: https://dl.fbaipublicfiles.com/vjepa2/
    """
    try:
        # Try to import V-JEPA 2
        from vjepa2 import create_model
        
        model = create_model(model_type)
        if weights_path:
            state_dict = torch.load(weights_path, map_location="cpu")
            model.load_state_dict(state_dict)
        
        model.eval()
        for p in model.parameters():
            p.requires_grad = False
        
        return model
    
    except ImportError:
        print("V-JEPA 2 not installed. Using placeholder encoder.")
        return None


def load_dinov2_encoder(
    model_type: str = "dinov2_vitl14"
) -> nn.Module:
    """
    Load DINOv2 encoder as fallback (frozen).
    """
    try:
        model = torch.hub.load('facebookresearch/dinov2', model_type)
        model.eval()
        for p in model.parameters():
            p.requires_grad = False
        return model
    
    except Exception as e:
        print(f"DINOv2 not available: {e}. Using placeholder encoder.")
        return None


class PlaceholderEncoder(nn.Module):
    """Identity encoder for pre-computed embeddings."""
    
    def __init__(self, embed_dim: int = 1024):
        super().__init__()
        self.embed_dim = embed_dim
    
    def forward(self, x):
        return x


# =============================================================================
# DATASET FOR VIDEO PAIRS
# =============================================================================

class VideoPairDataset(torch.utils.data.Dataset):
    """
    Dataset of (frame_t, frame_{t+k}) pairs for world model training.
    
    Supports both:
    - Pre-computed embeddings (recommended for Stage 1)
    - Raw frames (requires encoder forward pass)
    """
    
    def __init__(
        self,
        embeddings_path: Optional[str] = None,
        frames_dir: Optional[str] = None,
        horizon: int = 4,         # k frames ahead
        transform=None
    ):
        self.horizon = horizon
        self.transform = transform
        
        if embeddings_path and os.path.exists(embeddings_path):
            # Load pre-computed embeddings
            data = torch.load(embeddings_path)
            self.embeddings = data["embeddings"]  # [N, T, D]
            self.use_precomputed = True
            print(f"Loaded {len(self.embeddings)} video embeddings")
        else:
            self.embeddings = None
            self.use_precomputed = False
            # Would load from frames_dir for raw frames
    
    def __len__(self):
        if self.use_precomputed:
            # Each video can produce T - horizon pairs
            n_videos, T, _ = self.embeddings.shape
            return n_videos * (T - self.horizon)
        return 0
    
    def __getitem__(self, idx):
        if self.use_precomputed:
            n_videos, T, D = self.embeddings.shape
            pairs_per_video = T - self.horizon
            
            video_idx = idx // pairs_per_video
            frame_idx = idx % pairs_per_video
            
            current_emb = self.embeddings[video_idx, frame_idx]
            future_emb = self.embeddings[video_idx, frame_idx + self.horizon]
            
            return current_emb, future_emb
        
        raise NotImplementedError("Raw frame loading not implemented")


if __name__ == "__main__":
    # Quick sanity check
    print("CFC-JEPA World Model")
    print("=" * 50)
    
    config = CFCJEPAConfig(
        encoder_dim=512,  # Match our validated state_dim
        hidden_dim=512,
        num_layers=4,
    )
    
    model = CFCJEPAWorldModel(config)
    n_params = sum(p.numel() for p in model.parameters() if p.requires_grad)
    print(f"Trainable parameters: {n_params:,}")
    
    # Test forward
    B = 4
    current = torch.randn(B, config.encoder_dim)
    future = torch.randn(B, config.encoder_dim)
    
    losses = model.compute_total_loss(current, future)
    print(f"\nTest forward pass:")
    for k, v in losses.items():
        print(f"  {k}: {v.item():.4f}")
    
    print("\n✓ CFC-JEPA World Model ready")
