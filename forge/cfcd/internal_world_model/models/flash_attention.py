"""
Flash Attention Implementation for Internal World Model.
Memory-efficient attention for long sequences.
"""
import torch
import torch.nn as nn
import torch.nn.functional as F
from typing import Optional, Tuple
import math


class FlashAttention(nn.Module):
    """
    Memory-efficient attention using tiling.
    Falls back to standard attention if flash_attn is not available.
    """
    def __init__(self, dim: int, num_heads: int, dropout: float = 0.0, 
                 causal: bool = False):
        super().__init__()
        self.dim = dim
        self.num_heads = num_heads
        self.head_dim = dim // num_heads
        self.scale = self.head_dim ** -0.5
        self.causal = causal

        self.qkv = nn.Linear(dim, dim * 3, bias=False)
        self.out_proj = nn.Linear(dim, dim, bias=False)
        self.dropout = nn.Dropout(dropout)

        # Try to import flash_attn
        self.use_flash_attn = False
        try:
            from flash_attn import flash_attn_func
            self.flash_attn_func = flash_attn_func
            self.use_flash_attn = True
            print("Flash Attention enabled")
        except ImportError:
            print("Flash Attention not available, using standard attention")

    def forward(self, x: torch.Tensor, 
                attn_mask: Optional[torch.Tensor] = None) -> torch.Tensor:
        B, L, D = x.shape

        # QKV projection
        qkv = self.qkv(x).reshape(B, L, 3, self.num_heads, self.head_dim)
        qkv = qkv.permute(2, 0, 3, 1, 4)  # (3, B, num_heads, L, head_dim)
        q, k, v = qkv[0], qkv[1], qkv[2]

        if self.use_flash_attn and attn_mask is None:
            # Use Flash Attention
            # flash_attn expects: (B, L, num_heads, head_dim)
            q = q.transpose(1, 2)
            k = k.transpose(1, 2)
            v = v.transpose(1, 2)

            out = self.flash_attn_func(
                q, k, v,
                dropout_p=self.dropout.p if self.training else 0.0,
                causal=self.causal,
                softmax_scale=self.scale
            )
            out = out.transpose(1, 2)  # (B, num_heads, L, head_dim)
        else:
            # Standard attention
            scores = torch.matmul(q, k.transpose(-2, -1)) * self.scale

            if self.causal:
                causal_mask = torch.triu(torch.ones(L, L, device=x.device), diagonal=1).bool()
                scores.masked_fill_(causal_mask, float('-inf'))

            if attn_mask is not None:
                scores = scores + attn_mask

            attn = F.softmax(scores, dim=-1)
            attn = self.dropout(attn)
            out = torch.matmul(attn, v)

        # Reshape and project
        out = out.transpose(1, 2).reshape(B, L, D)
        out = self.out_proj(out)

        return out


class MemoryEfficientAttention(nn.Module):
    """
    Memory-efficient attention using gradient checkpointing and chunking.
    For very long sequences.
    """
    def __init__(self, dim: int, num_heads: int, dropout: float = 0.0,
                 chunk_size: int = 1024):
        super().__init__()
        self.dim = dim
        self.num_heads = num_heads
        self.head_dim = dim // num_heads
        self.chunk_size = chunk_size
        self.scale = self.head_dim ** -0.5

        self.q_proj = nn.Linear(dim, dim, bias=False)
        self.k_proj = nn.Linear(dim, dim, bias=False)
        self.v_proj = nn.Linear(dim, dim, bias=False)
        self.out_proj = nn.Linear(dim, dim, bias=False)
        self.dropout = nn.Dropout(dropout)

    def _chunked_attention(self, q: torch.Tensor, k: torch.Tensor, 
                           v: torch.Tensor) -> torch.Tensor:
        """Process attention in chunks to save memory."""
        B, H, L, D = q.shape
        chunk_size = min(self.chunk_size, L)

        outputs = []
        for i in range(0, L, chunk_size):
            q_chunk = q[:, :, i:i+chunk_size, :]

            # Compute attention for this chunk
            scores = torch.matmul(q_chunk, k.transpose(-2, -1)) * self.scale
            attn = F.softmax(scores, dim=-1)
            attn = self.dropout(attn)
            out_chunk = torch.matmul(attn, v)
            outputs.append(out_chunk)

        return torch.cat(outputs, dim=2)

    def forward(self, x: torch.Tensor, 
                context: Optional[torch.Tensor] = None) -> torch.Tensor:
        B, L, D = x.shape

        q = self.q_proj(x).view(B, L, self.num_heads, self.head_dim).transpose(1, 2)

        if context is None:
            k = self.k_proj(x).view(B, L, self.num_heads, self.head_dim).transpose(1, 2)
            v = self.v_proj(x).view(B, L, self.num_heads, self.head_dim).transpose(1, 2)
        else:
            _, S, _ = context.shape
            k = self.k_proj(context).view(B, S, self.num_heads, self.head_dim).transpose(1, 2)
            v = self.v_proj(context).view(B, S, self.num_heads, self.head_dim).transpose(1, 2)

        # Use chunked attention for long sequences
        if L > self.chunk_size:
            out = self._chunked_attention(q, k, v)
        else:
            scores = torch.matmul(q, k.transpose(-2, -1)) * self.scale
            attn = F.softmax(scores, dim=-1)
            attn = self.dropout(attn)
            out = torch.matmul(attn, v)

        out = out.transpose(1, 2).reshape(B, L, D)
        out = self.out_proj(out)

        return out


class RotaryPositionalEmbedding(nn.Module):
    """Rotary Position Embedding (RoPE) for better long-sequence modeling."""
    def __init__(self, dim: int, max_seq_len: int = 8192, base: float = 10000.0):
        super().__init__()
        self.dim = dim
        self.max_seq_len = max_seq_len
        self.base = base

        # Precompute frequency tensor
        inv_freq = 1.0 / (base ** (torch.arange(0, dim, 2).float() / dim))
        self.register_buffer('inv_freq', inv_freq)

    def forward(self, x: torch.Tensor, seq_len: int) -> torch.Tensor:
        """Apply RoPE to input tensor."""
        # Create position * frequency matrix
        t = torch.arange(seq_len, device=x.device).type_as(self.inv_freq)
        freqs = torch.einsum('i,j->ij', t, self.inv_freq)
        emb = torch.cat((freqs, freqs), dim=-1)

        cos_emb = emb.cos()[None, None, :, :]
        sin_emb = emb.sin()[None, None, :, :]

        # Apply rotation
        x1, x2 = x[..., ::2], x[..., 1::2]
        rotated = torch.stack([-x2, x1], dim=-1).flatten(-2)

        return x * cos_emb + rotated * sin_emb


class ScaledEncoderLayer(nn.Module):
    """Encoder layer with Flash Attention and optional RoPE."""
    def __init__(self, dim: int, num_heads: int, dropout: float = 0.1,
                 use_flash: bool = True, use_rope: bool = True,
                 mlp_ratio: float = 4.0):
        super().__init__()

        self.use_rope = use_rope

        # Attention
        if use_flash:
            self.attn = FlashAttention(dim, num_heads, dropout, causal=False)
        else:
            self.attn = MemoryEfficientAttention(dim, num_heads, dropout)

        # RoPE
        if use_rope:
            self.rope = RotaryPositionalEmbedding(dim // num_heads)

        # Layer norms (pre-norm)
        self.norm1 = nn.LayerNorm(dim)
        self.norm2 = nn.LayerNorm(dim)

        # MLP
        mlp_dim = int(dim * mlp_ratio)
        self.mlp = nn.Sequential(
            nn.Linear(dim, mlp_dim),
            nn.GELU(),
            nn.Dropout(dropout),
            nn.Linear(mlp_dim, dim),
            nn.Dropout(dropout)
        )

    def forward(self, x: torch.Tensor, 
                mask: Optional[torch.Tensor] = None) -> torch.Tensor:
        h = self.norm1(x)

        # Apply RoPE if enabled
        if self.use_rope:
            B, H, L, D = h.shape[0], 1, h.shape[1], h.shape[2]
            h_rope = h.view(B, L, H, D).transpose(1, 2)
            h_rope = self.rope(h_rope, L)
            h = h_rope.transpose(1, 2).reshape(B, L, D)

        x = x + self.attn(h, mask)
        x = x + self.mlp(self.norm2(x))
        return x
