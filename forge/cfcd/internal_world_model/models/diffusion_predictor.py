"""
Diffusion Predictor: Future State Prediction via Denoising Diffusion
Iteratively refines noisy future states into clear predictions.
"""
import torch
import torch.nn as nn
import torch.nn.functional as F
import math
from typing import Optional, Tuple, List, Dict
import numpy as np


class SinusoidalTimeEmbedding(nn.Module):
    """Sinusoidal embedding for diffusion timesteps."""
    def __init__(self, dim: int):
        super().__init__()
        self.dim = dim

    def forward(self, time: torch.Tensor) -> torch.Tensor:
        device = time.device
        half_dim = self.dim // 2
        embeddings = math.log(10000) / (half_dim - 1)
        embeddings = torch.exp(torch.arange(half_dim, device=device) * -embeddings)
        embeddings = time[:, None] * embeddings[None, :]
        embeddings = torch.cat((embeddings.sin(), embeddings.cos()), dim=-1)
        return embeddings


class DiffusionBlock(nn.Module):
    """Residual block with time conditioning for diffusion."""
    def __init__(self, dim: int, time_emb_dim: int, num_heads: int = 8):
        super().__init__()

        self.time_mlp = nn.Sequential(
            nn.SiLU(),
            nn.Linear(time_emb_dim, dim)
        )

        self.norm1 = nn.LayerNorm(dim)
        self.attn = nn.MultiheadAttention(dim, num_heads, batch_first=True)

        self.norm2 = nn.LayerNorm(dim)
        self.ffn = nn.Sequential(
            nn.Linear(dim, dim * 4),
            nn.GELU(),
            nn.Linear(dim * 4, dim)
        )

        self.residual_scale = nn.Parameter(torch.ones(1))

    def forward(self, x: torch.Tensor, t_emb: torch.Tensor, 
                context: Optional[torch.Tensor] = None) -> torch.Tensor:
        # Time conditioning
        t = self.time_mlp(t_emb)

        # Self-attention with time bias
        h = self.norm1(x)
        h = h + t.unsqueeze(1)  # Add time embedding
        h_attn, _ = self.attn(h, h, h)
        x = x + h_attn

        # Cross-attention to context (current state)
        if context is not None:
            h = self.norm1(x)
            h_cross, _ = self.attn(h, context, context)
            x = x + h_cross

        # FFN
        h = self.norm2(x)
        h = self.ffn(h)
        x = x + h * self.residual_scale

        return x


class CFCDiffusionBlock(nn.Module):
    """
    CFC-enhanced residual block with Theta/Gamma streams.
    
    Per CFC-World Model Engineering Spec:
    - Theta Stream (Context): Compress → Causal Conv1D → Gate logits
    - Gamma Stream (Denoising): Attention + FFN transform
    - Soft Gating: modulation = 1.0 + 0.5 × tanh(gate_logits) ∈ [0.5, 1.5]
    """
    def __init__(self, dim: int, time_emb_dim: int, num_heads: int = 8,
                 compress_ratio: int = 4, kernel_size: int = 4):
        super().__init__()
        self.dim = dim
        self.compress_dim = dim // compress_ratio
        self.kernel_size = kernel_size
        
        # === THETA STREAM (Context Integration) ===
        # Step 1: Compress D → D/4
        self.theta_compress = nn.Linear(dim, self.compress_dim)
        # Step 2: Causal depthwise Conv1D over time (integrates [t, t-1, t-2, t-3])
        self.theta_conv = nn.Conv1d(
            self.compress_dim, self.compress_dim,
            kernel_size=kernel_size,
            padding=kernel_size - 1,  # Causal padding
            groups=self.compress_dim   # Depthwise
        )
        # Step 3: Expand to gate logits D/4 → D
        self.theta_expand = nn.Linear(self.compress_dim, dim)
        
        # === GAMMA STREAM (Denoising) ===
        self.time_mlp = nn.Sequential(
            nn.SiLU(),
            nn.Linear(time_emb_dim, dim)
        )
        self.norm1 = nn.LayerNorm(dim)
        self.attn = nn.MultiheadAttention(dim, num_heads, batch_first=True)
        self.norm2 = nn.LayerNorm(dim)
        self.ffn = nn.Sequential(
            nn.Linear(dim, dim * 4),
            nn.GELU(),
            nn.Linear(dim * 4, dim)
        )
        self.residual_scale = nn.Parameter(torch.ones(1))
        
        # Gate statistics tracking for diagnostics
        self.register_buffer('gate_values', torch.tensor([]))
        self.track_gates = True
        
    def forward(self, x: torch.Tensor, t_emb: torch.Tensor,
                context: Optional[torch.Tensor] = None,
                track_gates: bool = True) -> Tuple[torch.Tensor, Optional[torch.Tensor]]:
        """
        Args:
            x: (batch, seq, dim) input tensor
            t_emb: (batch, time_emb_dim) time embedding
            context: optional (batch, ctx_len, dim) cross-attention context
            track_gates: whether to track gate statistics
            
        Returns:
            output: (batch, seq, dim)
            modulation: (batch, seq, dim) gate modulation values for diagnostics
        """
        batch, seq, dim = x.shape
        
        # === THETA STREAM ===
        # Compress
        theta = self.theta_compress(x)  # (B, T, D/4)
        # Causal Conv1D: transpose for conv, apply, truncate to causal
        theta = theta.transpose(1, 2)   # (B, D/4, T)
        theta = self.theta_conv(theta)  # (B, D/4, T + k - 1)
        theta = theta[:, :, :seq]       # Causal: only use past (B, D/4, T)
        theta = theta.transpose(1, 2)   # (B, T, D/4)
        # Expand to gate logits
        gate_logits = self.theta_expand(theta)  # (B, T, D)
        
        # === GAMMA STREAM (existing denoising) ===
        # Time conditioning
        t = self.time_mlp(t_emb)
        
        # Self-attention with time bias
        h = self.norm1(x)
        h = h + t.unsqueeze(1)
        h_attn, _ = self.attn(h, h, h)
        gamma = x + h_attn
        
        # Cross-attention to context (current state)
        if context is not None:
            h = self.norm1(gamma)
            h_cross, _ = self.attn(h, context, context)
            gamma = gamma + h_cross
        
        # FFN
        h = self.norm2(gamma)
        h = self.ffn(h)
        gamma = gamma + h * self.residual_scale
        
        # === SOFT GATING ===
        # Modulation ∈ [0.5, 1.5] — never fully silences any dimension
        modulation = 1.0 + 0.5 * torch.tanh(gate_logits)
        output = gamma * modulation
        
        # Track gate statistics for diagnostics
        # NOTE: we want telemetry during eval/validation too, not just training.
        if track_gates:
            with torch.no_grad():
                self.gate_values = modulation.detach().flatten()
        
        return output, modulation
    
    def get_gate_stats(self) -> Tuple[float, float, float, float]:
        """Returns (mean, std, min, max) of recent gate values."""
        if self.gate_values.numel() == 0:
            return 1.0, 0.0, 1.0, 1.0
        return (
            self.gate_values.mean().item(),
            self.gate_values.std().item(),
            self.gate_values.min().item(),
            self.gate_values.max().item()
        )
    
    def reset_gate_stats(self):
        """Reset gate tracking buffer."""
        self.gate_values = torch.tensor([], device=self.gate_values.device)


class DiffusionPredictor(nn.Module):
    """
    Diffusion-based future state predictor.
    Learns to denoise future states conditioned on current state.
    """
    def __init__(
        self,
        state_dim: int = 512,
        hidden_dim: int = 1024,
        num_layers: int = 8,
        num_heads: int = 8,
        num_timesteps: int = 1000,
        beta_schedule: str = 'cosine',
        prediction_type: str = 'epsilon',  # 'epsilon' or 'v_prediction'
        # CFC parameters
        use_cfc: bool = True,
        cfc_compress_ratio: int = 4,
        cfc_kernel_size: int = 4
    ):
        super().__init__()

        self.state_dim = state_dim
        self.hidden_dim = hidden_dim
        self.num_timesteps = num_timesteps
        self.prediction_type = prediction_type
        self.use_cfc = use_cfc

        # Time embedding
        time_emb_dim = hidden_dim
        self.time_embed = nn.Sequential(
            SinusoidalTimeEmbedding(time_emb_dim),
            nn.Linear(time_emb_dim, time_emb_dim * 4),
            nn.SiLU(),
            nn.Linear(time_emb_dim * 4, time_emb_dim)
        )

        # Input projection
        self.input_proj = nn.Linear(state_dim, hidden_dim)

        # Context projection (for current state conditioning)
        self.context_proj = nn.Linear(state_dim, hidden_dim)

        # Diffusion blocks (CFC or baseline)
        if use_cfc:
            self.blocks = nn.ModuleList([
                CFCDiffusionBlock(hidden_dim, time_emb_dim, num_heads,
                                  cfc_compress_ratio, cfc_kernel_size)
                for _ in range(num_layers)
            ])
        else:
            self.blocks = nn.ModuleList([
                DiffusionBlock(hidden_dim, time_emb_dim, num_heads)
                for _ in range(num_layers)
            ])

        # Output projection
        self.output_proj = nn.Sequential(
            nn.LayerNorm(hidden_dim),
            nn.Linear(hidden_dim, hidden_dim),
            nn.SiLU(),
            nn.Linear(hidden_dim, state_dim)
        )

        # Setup noise schedule
        self._setup_noise_schedule(beta_schedule)

    def _setup_noise_schedule(self, schedule: str):
        """Setup diffusion noise schedule."""
        if schedule == 'linear':
            self.betas = torch.linspace(1e-4, 0.02, self.num_timesteps)
        elif schedule == 'cosine':
            self.betas = self._cosine_beta_schedule(self.num_timesteps)
        else:
            raise ValueError(f"Unknown schedule: {schedule}")

        self.alphas = 1.0 - self.betas
        self.alphas_cumprod = torch.cumprod(self.alphas, dim=0)
        self.alphas_cumprod_prev = F.pad(self.alphas_cumprod[:-1], (1, 0), value=1.0)

        # Register as buffers
        self.register_buffer('betas_tensor', self.betas)
        self.register_buffer('alphas_tensor', self.alphas)
        self.register_buffer('alphas_cumprod_tensor', self.alphas_cumprod)
        self.register_buffer('alphas_cumprod_prev_tensor', self.alphas_cumprod_prev)

        # Calculations for diffusion q(x_t | x_{t-1})
        self.register_buffer('sqrt_alphas_cumprod', torch.sqrt(self.alphas_cumprod))
        self.register_buffer('sqrt_one_minus_alphas_cumprod', 
                            torch.sqrt(1.0 - self.alphas_cumprod))

        # Calculations for posterior q(x_{t-1} | x_t, x_0)
        self.register_buffer('sqrt_recip_alphas_cumprod', 
                            torch.sqrt(1.0 / self.alphas_cumprod))
        self.register_buffer('sqrt_recipm1_alphas_cumprod',
                            torch.sqrt(1.0 / self.alphas_cumprod - 1))

        self.register_buffer('posterior_variance',
                            self.betas * (1.0 - self.alphas_cumprod_prev) / 
                            (1.0 - self.alphas_cumprod))
        self.register_buffer('posterior_log_variance_clipped',
                            torch.log(torch.clamp(self.posterior_variance, min=1e-20)))
        self.register_buffer('posterior_mean_coef1',
                            self.betas * torch.sqrt(self.alphas_cumprod_prev) / 
                            (1.0 - self.alphas_cumprod))
        self.register_buffer('posterior_mean_coef2',
                            (1.0 - self.alphas_cumprod_prev) * torch.sqrt(self.alphas) / 
                            (1.0 - self.alphas_cumprod))

    def _cosine_beta_schedule(self, timesteps: int, s: float = 0.008) -> torch.Tensor:
        """Cosine schedule as proposed in Improved Denoising Diffusion Probabilistic Models."""
        steps = timesteps + 1
        x = torch.linspace(0, timesteps, steps)
        alphas_cumprod = torch.cos(((x / timesteps) + s) / (1 + s) * math.pi * 0.5) ** 2
        alphas_cumprod = alphas_cumprod / alphas_cumprod[0]
        betas = 1 - (alphas_cumprod[1:] / alphas_cumprod[:-1])
        return torch.clip(betas, 0.0001, 0.9999)

    def q_sample(self, x_0: torch.Tensor, t: torch.Tensor, 
                 noise: Optional[torch.Tensor] = None) -> torch.Tensor:
        """
        Forward diffusion process: q(x_t | x_0)
        Add noise to data according to schedule.
        """
        if noise is None:
            noise = torch.randn_like(x_0)

        sqrt_alphas_cumprod_t = self.sqrt_alphas_cumprod[t].view(-1, 1)
        sqrt_one_minus_alphas_cumprod_t = self.sqrt_one_minus_alphas_cumprod[t].view(-1, 1)

        return sqrt_alphas_cumprod_t * x_0 + sqrt_one_minus_alphas_cumprod_t * noise

    def predict_noise(self, x_t: torch.Tensor, t: torch.Tensor, 
                      context: torch.Tensor, track_gates: bool = True) -> torch.Tensor:
        """
        Predict noise in x_t given context (current state).
        """
        # Time embedding
        t_emb = self.time_embed(t.float())

        # Project inputs
        h = self.input_proj(x_t)
        ctx = self.context_proj(context).unsqueeze(1)  # (B, 1, hidden_dim)

        # Apply diffusion blocks
        for block in self.blocks:
            h_in = h.unsqueeze(1) if h.dim() == 2 else h
            out = block(h_in, t_emb, ctx) if not self.use_cfc else block(h_in, t_emb, ctx, track_gates)
            # CFC blocks return (output, modulation), baseline blocks return output
            h = out[0] if isinstance(out, tuple) else out
            h = h.squeeze(1) if h.dim() == 3 and h.size(1) == 1 else h

        # Output projection
        h = h.squeeze(1) if h.dim() == 3 else h
        return self.output_proj(h)

    def forward(self, x_0: torch.Tensor, context: torch.Tensor, 
                t: Optional[torch.Tensor] = None) -> Tuple[torch.Tensor, torch.Tensor]:
        """
        Training forward pass.

        Args:
            x_0: Clean future state (B, state_dim)
            context: Current state for conditioning (B, state_dim)
            t: Timesteps (B,), randomly sampled if None

        Returns:
            predicted_noise, true_noise
        """
        batch_size = x_0.shape[0]
        device = x_0.device

        # Sample random timesteps
        if t is None:
            t = torch.randint(0, self.num_timesteps, (batch_size,), device=device)

        # Sample noise
        noise = torch.randn_like(x_0)

        # Forward diffusion: get noisy x_t
        x_t = self.q_sample(x_0, t, noise)

        # Predict noise
        pred_noise = self.predict_noise(x_t, t, context)

        return pred_noise, noise

    def p_mean_variance(self, x_t: torch.Tensor, t: torch.Tensor, 
                        context: torch.Tensor) -> Tuple[torch.Tensor, torch.Tensor]:
        """
        Compute mean and variance of p(x_{t-1} | x_t).
        """
        # Predict noise
        pred_noise = self.predict_noise(x_t, t, context)

        # Compute predicted x_0
        if self.prediction_type == 'epsilon':
            pred_x_0 = (self.sqrt_recip_alphas_cumprod[t].view(-1, 1) * x_t - 
                       self.sqrt_recipm1_alphas_cumprod[t].view(-1, 1) * pred_noise)
        else:  # v_prediction
            # v = alpha * noise - sqrt(1-alpha) * x_0
            pred_x_0 = (self.sqrt_alphas_cumprod[t].view(-1, 1) * x_t - 
                       self.sqrt_one_minus_alphas_cumprod[t].view(-1, 1) * pred_noise)

        pred_x_0 = torch.clamp(pred_x_0, -10, 10)

        # Compute posterior mean
        model_mean = (self.posterior_mean_coef1[t].view(-1, 1) * pred_x_0 +
                     self.posterior_mean_coef2[t].view(-1, 1) * x_t)

        model_variance = self.posterior_variance[t].view(-1, 1)
        model_log_variance = self.posterior_log_variance_clipped[t].view(-1, 1)

        return model_mean, model_variance, model_log_variance

    @torch.no_grad()
    def p_sample(self, x_t: torch.Tensor, t: torch.Tensor, 
                 context: torch.Tensor) -> torch.Tensor:
        """
        Sample x_{t-1} from p(x_{t-1} | x_t).
        """
        model_mean, _, model_log_variance = self.p_mean_variance(x_t, t, context)

        noise = torch.randn_like(x_t)
        # No noise when t == 0
        nonzero_mask = (t != 0).float().view(-1, 1)

        return model_mean + nonzero_mask * torch.exp(0.5 * model_log_variance) * noise

    @torch.no_grad()
    def sample(self, context: torch.Tensor, num_samples: int = 1,
               return_trajectory: bool = False) -> torch.Tensor:
        """
        Generate future state by denoising from random noise.

        Args:
            context: Current state (B, state_dim)
            num_samples: Number of future states to generate per context
            return_trajectory: If True, return all intermediate states

        Returns:
            Generated future state(s), optionally with trajectory
        """
        batch_size = context.shape[0]
        device = context.device

        # Expand context for multiple samples
        if num_samples > 1:
            context = context.unsqueeze(1).repeat(1, num_samples, 1)
            context = context.view(-1, self.state_dim)
            batch_size = batch_size * num_samples

        # Start from pure noise
        x_t = torch.randn(batch_size, self.state_dim, device=device)

        trajectory = [x_t] if return_trajectory else None

        # Iterative denoising
        for t in reversed(range(self.num_timesteps)):
            t_batch = torch.full((batch_size,), t, device=device, dtype=torch.long)
            x_t = self.p_sample(x_t, t_batch, context)

            if return_trajectory:
                trajectory.append(x_t)

        # Reshape if multiple samples
        if num_samples > 1:
            original_batch = context.shape[0] // num_samples
            x_t = x_t.view(original_batch, num_samples, self.state_dim)

        if return_trajectory:
            return x_t, torch.stack(trajectory, dim=0)
        return x_t

    @torch.no_grad()
    def ddim_sample(self, context: torch.Tensor, num_samples: int = 1,
                    ddim_timesteps: int = 50, eta: float = 0.0) -> torch.Tensor:
        """
        Fast sampling using DDIM (Denoising Diffusion Implicit Models).
        Much faster than standard sampling.

        Args:
            context: Current state (B, state_dim)
            num_samples: Number of future states to generate
            ddim_timesteps: Number of sampling steps (fewer = faster)
            eta: Stochasticity parameter (0 = deterministic)
        """
        batch_size = context.shape[0]
        device = context.device

        if num_samples > 1:
            context = context.unsqueeze(1).repeat(1, num_samples, 1)
            context = context.view(-1, self.state_dim)
            batch_size = batch_size * num_samples

        # Create subsequence of timesteps
        c = self.num_timesteps // ddim_timesteps
        timesteps = np.asarray(list(range(0, self.num_timesteps, c))) + 1
        timesteps = timesteps[:ddim_timesteps]
        timesteps = torch.from_numpy(timesteps).long().to(device)

        # Start from noise
        x_t = torch.randn(batch_size, self.state_dim, device=device)

        # Reverse diffusion
        for i in reversed(range(len(timesteps))):
            t = torch.full((batch_size,), timesteps[i], device=device, dtype=torch.long)

            # Predict noise
            pred_noise = self.predict_noise(x_t, t, context)

            # Get alpha values
            alpha_t = self.alphas_cumprod_tensor[t].view(-1, 1)

            if i > 0:
                alpha_t_prev = self.alphas_cumprod_tensor[timesteps[i - 1]].view(-1, 1)
            else:
                alpha_t_prev = torch.ones_like(alpha_t)

            # Predict x_0
            pred_x_0 = (x_t - torch.sqrt(1 - alpha_t) * pred_noise) / torch.sqrt(alpha_t)

            # Compute direction pointing to x_t
            dir_xt = torch.sqrt(1 - alpha_t_prev - eta**2 * 
                               (1 - alpha_t_prev) / (1 - alpha_t) * (1 - alpha_t / alpha_t_prev)) * pred_noise

            # Random noise
            noise = torch.randn_like(x_t) if eta > 0 else 0
            sigma_t = eta * torch.sqrt((1 - alpha_t_prev) / (1 - alpha_t)) * torch.sqrt(1 - alpha_t / alpha_t_prev)

            # Compute x_{t-1}
            x_t = torch.sqrt(alpha_t_prev) * pred_x_0 + dir_xt + sigma_t * noise

        if num_samples > 1:
            original_batch = context.shape[0] // num_samples
            x_t = x_t.view(original_batch, num_samples, self.state_dim)

        return x_t

    def compute_loss(self, x_0: torch.Tensor, context: torch.Tensor) -> torch.Tensor:
        """Compute training loss."""
        pred_noise, true_noise = self.forward(x_0, context)
        return F.mse_loss(pred_noise, true_noise)

    def get_gate_stats(self) -> dict:
        """
        Get gate statistics from all CFC blocks.
        Returns dict with per-layer stats: {layer_idx: (mean, std, min, max)}
        """
        if not self.use_cfc:
            return {}
        stats = {}
        for i, block in enumerate(self.blocks):
            if hasattr(block, 'get_gate_stats'):
                stats[i] = block.get_gate_stats()
        return stats

    def reset_gate_stats(self):
        """Reset gate tracking in all CFC blocks."""
        if self.use_cfc:
            for block in self.blocks:
                if hasattr(block, 'reset_gate_stats'):
                    block.reset_gate_stats()
