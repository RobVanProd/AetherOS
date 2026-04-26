"""
JEPA Encoder: Joint Embedding Predictive Architecture
Compresses sensory data into abstract state representations.
"""
import torch
import torch.nn as nn
import torch.nn.functional as F
from typing import Dict, Tuple, Optional
import math


class SinusoidalPositionalEncoding(nn.Module):
    """Sinusoidal positional encoding for sequences."""
    def __init__(self, d_model: int, max_len: int = 5000):
        super().__init__()
        position = torch.arange(max_len).unsqueeze(1)
        div_term = torch.exp(torch.arange(0, d_model, 2) * (-math.log(10000.0) / d_model))
        pe = torch.zeros(max_len, d_model)
        pe[:, 0::2] = torch.sin(position * div_term)
        pe[:, 1::2] = torch.cos(position * div_term)
        self.register_buffer('pe', pe)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return x + self.pe[:x.size(1)]


class VisionEncoder(nn.Module):
    """Vision encoder for processing image inputs."""
    def __init__(
        self, 
        image_size: int = 224,
        patch_size: int = 16,
        in_channels: int = 3,
        embed_dim: int = 768,
        num_layers: int = 12,
        num_heads: int = 12
    ):
        super().__init__()
        self.patch_size = patch_size
        self.num_patches = (image_size // patch_size) ** 2

        # Patch embedding
        self.patch_embed = nn.Conv2d(
            in_channels, embed_dim, 
            kernel_size=patch_size, 
            stride=patch_size
        )

        # Positional encoding
        self.pos_embed = nn.Parameter(torch.zeros(1, self.num_patches, embed_dim))
        nn.init.trunc_normal_(self.pos_embed, std=0.02)

        # Transformer encoder
        encoder_layer = nn.TransformerEncoderLayer(
            d_model=embed_dim,
            nhead=num_heads,
            dim_feedforward=embed_dim * 4,
            dropout=0.1,
            activation='gelu',
            batch_first=True
        )
        self.transformer = nn.TransformerEncoder(encoder_layer, num_layers)

        # Layer norm
        self.norm = nn.LayerNorm(embed_dim)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        # x: (B, C, H, W)
        B = x.shape[0]

        # Patch embedding: (B, embed_dim, H//patch, W//patch)
        x = self.patch_embed(x)

        # Flatten patches: (B, num_patches, embed_dim)
        x = x.flatten(2).transpose(1, 2)

        # Add positional embeddings
        x = x + self.pos_embed

        # Transformer encoding
        x = self.transformer(x)
        x = self.norm(x)

        # Global average pooling
        x = x.mean(dim=1)  # (B, embed_dim)

        return x


class TextEncoder(nn.Module):
    """Text encoder for processing language inputs."""
    def __init__(
        self,
        vocab_size: int = 50000,
        embed_dim: int = 768,
        num_layers: int = 12,
        num_heads: int = 12,
        max_seq_len: int = 512,
        dropout: float = 0.1
    ):
        super().__init__()

        self.token_embed = nn.Embedding(vocab_size, embed_dim)
        self.pos_encoding = SinusoidalPositionalEncoding(embed_dim, max_seq_len)
        self.dropout = nn.Dropout(dropout)

        # Use custom transformer blocks instead of nn.TransformerEncoder
        self.layers = nn.ModuleList([
            TransformerEncoderLayer(embed_dim, num_heads, dropout)
            for _ in range(num_layers)
        ])

        self.norm = nn.LayerNorm(embed_dim)

    def forward(self, x: torch.Tensor, mask: Optional[torch.Tensor] = None) -> torch.Tensor:
        # x: (B, seq_len)
        x = self.token_embed(x)
        x = self.pos_encoding(x)
        x = self.dropout(x)

        # Apply transformer layers with proper mask handling
        for layer in self.layers:
            x = layer(x, mask)

        x = self.norm(x)

        # Global average pooling (ignoring padding)
        if mask is not None:
            # mask: 1 for valid, 0 for padding
            mask_expanded = mask.unsqueeze(-1).float()
            x = (x * mask_expanded).sum(dim=1) / mask_expanded.sum(dim=1).clamp(min=1)
        else:
            x = x.mean(dim=1)

        return x


class TransformerEncoderLayer(nn.Module):
    """Custom transformer encoder layer with proper mask support."""
    def __init__(self, d_model: int, num_heads: int, dropout: float = 0.1):
        super().__init__()

        self.self_attn = nn.MultiheadAttention(
            d_model, num_heads, dropout=dropout, batch_first=True
        )

        self.linear1 = nn.Linear(d_model, d_model * 4)
        self.dropout = nn.Dropout(dropout)
        self.linear2 = nn.Linear(d_model * 4, d_model)

        self.norm1 = nn.LayerNorm(d_model)
        self.norm2 = nn.LayerNorm(d_model)

        self.dropout1 = nn.Dropout(dropout)
        self.dropout2 = nn.Dropout(dropout)

    def forward(self, src: torch.Tensor, mask: Optional[torch.Tensor] = None) -> torch.Tensor:
        # Self-attention
        src2 = self.norm1(src)

        # Convert mask for MultiheadAttention (True = mask out)
        attn_mask = None
        if mask is not None:
            attn_mask = (mask == 0)  # Invert: 0 in input -> True (masked)

        src2, _ = self.self_attn(src2, src2, src2, key_padding_mask=attn_mask)
        src = src + self.dropout1(src2)

        # Feed-forward
        src2 = self.norm2(src)
        src2 = self.linear2(self.dropout(F.gelu(self.linear1(src2))))
        src = src + self.dropout2(src2)

        return src


class JEPAEncoder(nn.Module):
    """
    Joint Embedding Predictive Architecture Encoder.
    Compresses sensory data (vision, text, or multimodal) into abstract states.
    """
    def __init__(
        self,
        state_dim: int = 512,
        vision_config: Optional[Dict] = None,
        text_config: Optional[Dict] = None,
        multimodal: bool = True
    ):
        super().__init__()

        self.state_dim = state_dim
        self.multimodal = multimodal

        # Vision encoder
        if vision_config is None:
            vision_config = {
                'image_size': 224,
                'patch_size': 16,
                'in_channels': 3,
                'embed_dim': 768,
                'num_layers': 6,
                'num_heads': 12
            }
        self.vision_encoder = VisionEncoder(**vision_config)

        # Text encoder
        if text_config is None:
            text_config = {
                'vocab_size': 50000,
                'embed_dim': 768,
                'num_layers': 6,
                'num_heads': 12,
                'max_seq_len': 512
            }
        self.text_encoder = TextEncoder(**text_config)

        # Projection layers to state space
        vision_embed_dim = vision_config['embed_dim']
        text_embed_dim = text_config['embed_dim']

        self.vision_proj = nn.Sequential(
            nn.Linear(vision_embed_dim, state_dim * 2),
            nn.LayerNorm(state_dim * 2),
            nn.GELU(),
            nn.Linear(state_dim * 2, state_dim)
        )

        self.text_proj = nn.Sequential(
            nn.Linear(text_embed_dim, state_dim * 2),
            nn.LayerNorm(state_dim * 2),
            nn.GELU(),
            nn.Linear(state_dim * 2, state_dim)
        )

        # Multimodal fusion (if both modalities present)
        if multimodal:
            self.fusion = nn.Sequential(
                nn.Linear(state_dim * 2, state_dim * 2),
                nn.LayerNorm(state_dim * 2),
                nn.GELU(),
                nn.Linear(state_dim * 2, state_dim)
            )

        # State refinement (VicReg-style variance/invariance/covariance)
        self.variance_coeff = 25.0
        self.covariance_coeff = 1.0

    def encode_vision(self, images: torch.Tensor) -> torch.Tensor:
        """Encode images to abstract state."""
        features = self.vision_encoder(images)
        state = self.vision_proj(features)
        return F.normalize(state, dim=-1)  # L2 normalize

    def encode_text(self, tokens: torch.Tensor, mask: Optional[torch.Tensor] = None) -> torch.Tensor:
        """Encode text to abstract state."""
        features = self.text_encoder(tokens, mask)
        state = self.text_proj(features)
        return F.normalize(state, dim=-1)  # L2 normalize

    def forward(
        self, 
        images: Optional[torch.Tensor] = None,
        text_tokens: Optional[torch.Tensor] = None,
        text_mask: Optional[torch.Tensor] = None
    ) -> Dict[str, torch.Tensor]:
        """
        Forward pass supporting unimodal or multimodal inputs.

        Returns:
            Dict with 'state' (abstract state) and optionally 'vision_state', 'text_state'
        """
        outputs = {}

        if images is not None and text_tokens is not None and self.multimodal:
            # Multimodal encoding
            vision_state = self.encode_vision(images)
            text_state = self.encode_text(text_tokens, text_mask)

            # Fusion
            combined = torch.cat([vision_state, text_state], dim=-1)
            state = self.fusion(combined)
            state = F.normalize(state, dim=-1)

            outputs['vision_state'] = vision_state
            outputs['text_state'] = text_state
            outputs['state'] = state

        elif images is not None:
            # Vision-only
            state = self.encode_vision(images)
            outputs['state'] = state
            outputs['vision_state'] = state

        elif text_tokens is not None:
            # Text-only
            state = self.encode_text(text_tokens, text_mask)
            outputs['state'] = state
            outputs['text_state'] = state

        else:
            raise ValueError("At least one of images or text_tokens must be provided")

        return outputs

    def compute_vicreg_loss(
        self, 
        state1: torch.Tensor, 
        state2: torch.Tensor
    ) -> Dict[str, torch.Tensor]:
        """
        Compute VICReg-style self-supervised loss.
        Used for training the encoder without labels.
        """
        # Invariance loss (MSE between embeddings)
        inv_loss = F.mse_loss(state1, state2)

        # Variance loss (maintain variance across batch)
        std1 = torch.sqrt(state1.var(dim=0) + 1e-4)
        std2 = torch.sqrt(state2.var(dim=0) + 1e-4)
        var_loss = torch.mean(F.relu(1 - std1)) + torch.mean(F.relu(1 - std2))

        # Covariance loss (decorrelate dimensions)
        def off_diagonal(x):
            n, m = x.shape
            return x.flatten()[:-1].view(n - 1, n + 1)[:, 1:].flatten()

        cov1 = (state1.T @ state1) / (state1.size(0) - 1)
        cov2 = (state2.T @ state2) / (state2.size(0) - 1)
        cov_loss = off_diagonal(cov1).pow_(2).sum() / state1.size(1)
        cov_loss += off_diagonal(cov2).pow_(2).sum() / state2.size(1)

        total_loss = inv_loss + self.variance_coeff * var_loss + self.covariance_coeff * cov_loss

        return {
            'total': total_loss,
            'invariance': inv_loss,
            'variance': var_loss,
            'covariance': cov_loss
        }
