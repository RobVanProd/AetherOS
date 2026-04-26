"""
LLM Decoder: State-to-Language Translation
Translates abstract future states into natural language or code.
"""
import torch
import torch.nn as nn
import torch.nn.functional as F
from typing import Optional, Dict, List, Tuple
import math


class StateConditionedAttention(nn.Module):
    """Cross-attention layer conditioned on state embeddings."""
    def __init__(self, dim: int, num_heads: int = 8, dropout: float = 0.1):
        super().__init__()
        self.num_heads = num_heads
        self.head_dim = dim // num_heads
        self.scale = self.head_dim ** -0.5

        self.q_proj = nn.Linear(dim, dim)
        self.k_proj = nn.Linear(dim, dim)
        self.v_proj = nn.Linear(dim, dim)
        self.out_proj = nn.Linear(dim, dim)

        self.dropout = nn.Dropout(dropout)

    def forward(self, query: torch.Tensor, state: torch.Tensor,
                attention_mask: Optional[torch.Tensor] = None) -> torch.Tensor:
        B, L, D = query.shape
        _, S, _ = state.shape

        # Project queries from input, keys/values from state
        Q = self.q_proj(query).view(B, L, self.num_heads, self.head_dim).transpose(1, 2)
        K = self.k_proj(state).view(B, S, self.num_heads, self.head_dim).transpose(1, 2)
        V = self.v_proj(state).view(B, S, self.num_heads, self.head_dim).transpose(1, 2)

        # Attention
        scores = torch.matmul(Q, K.transpose(-2, -1)) * self.scale

        if attention_mask is not None:
            scores = scores.masked_fill(attention_mask.unsqueeze(1).unsqueeze(1) == 0, float('-inf'))

        attn = F.softmax(scores, dim=-1)
        attn = self.dropout(attn)

        out = torch.matmul(attn, V)
        out = out.transpose(1, 2).contiguous().view(B, L, D)
        out = self.out_proj(out)

        return out


class DecoderLayer(nn.Module):
    """Transformer decoder layer with state conditioning."""
    def __init__(self, dim: int, num_heads: int = 8, dropout: float = 0.1):
        super().__init__()

        # Self-attention
        self.self_attn = nn.MultiheadAttention(dim, num_heads, dropout, batch_first=True)
        self.norm1 = nn.LayerNorm(dim)

        # Cross-attention to state
        self.cross_attn = StateConditionedAttention(dim, num_heads, dropout)
        self.norm2 = nn.LayerNorm(dim)

        # Feed-forward
        self.ffn = nn.Sequential(
            nn.Linear(dim, dim * 4),
            nn.GELU(),
            nn.Dropout(dropout),
            nn.Linear(dim * 4, dim),
            nn.Dropout(dropout)
        )
        self.norm3 = nn.LayerNorm(dim)

    def forward(self, x: torch.Tensor, state: torch.Tensor,
                self_attn_mask: Optional[torch.Tensor] = None,
                cross_attn_mask: Optional[torch.Tensor] = None) -> torch.Tensor:
        # Self-attention with causal mask
        h = self.norm1(x)
        h, _ = self.self_attn(h, h, h, attn_mask=self_attn_mask, need_weights=False)
        x = x + h

        # Cross-attention to state
        h = self.norm2(x)
        h = self.cross_attn(h, state, cross_attn_mask)
        x = x + h

        # Feed-forward
        h = self.norm3(x)
        h = self.ffn(h)
        x = x + h

        return x


class LLMDecoder(nn.Module):
    """
    Large Language Model Decoder conditioned on abstract states.
    Translates predicted future states into language or code.
    """
    def __init__(
        self,
        vocab_size: int = 50000,
        state_dim: int = 512,
        embed_dim: int = 768,
        num_layers: int = 12,
        num_heads: int = 12,
        max_seq_len: int = 512,
        dropout: float = 0.1,
        pad_token_id: int = 0
    ):
        super().__init__()

        self.vocab_size = vocab_size
        self.state_dim = state_dim
        self.embed_dim = embed_dim
        self.max_seq_len = max_seq_len
        self.pad_token_id = pad_token_id

        # Token embeddings
        self.token_embed = nn.Embedding(vocab_size, embed_dim, padding_idx=pad_token_id)

        # Positional embeddings (learned)
        self.pos_embed = nn.Embedding(max_seq_len, embed_dim)

        # State projection
        self.state_proj = nn.Sequential(
            nn.Linear(state_dim, embed_dim * 2),
            nn.LayerNorm(embed_dim * 2),
            nn.GELU(),
            nn.Linear(embed_dim * 2, embed_dim)
        )

        # Dropout
        self.dropout = nn.Dropout(dropout)

        # Decoder layers
        self.layers = nn.ModuleList([
            DecoderLayer(embed_dim, num_heads, dropout)
            for _ in range(num_layers)
        ])

        # Final layer norm
        self.norm = nn.LayerNorm(embed_dim)

        # Output projection
        self.lm_head = nn.Linear(embed_dim, vocab_size, bias=False)

        # Tie weights
        self.lm_head.weight = self.token_embed.weight

        # Initialize weights
        self._init_weights()

        # Pre-compute causal mask
        self.register_buffer('causal_mask', self._create_causal_mask(max_seq_len))

    def _init_weights(self):
        """Initialize weights."""
        nn.init.normal_(self.token_embed.weight, std=0.02)
        nn.init.normal_(self.pos_embed.weight, std=0.02)

        for module in self.modules():
            if isinstance(module, nn.Linear):
                nn.init.normal_(module.weight, std=0.02)
                if module.bias is not None:
                    nn.init.zeros_(module.bias)
            elif isinstance(module, nn.LayerNorm):
                nn.init.ones_(module.weight)
                nn.init.zeros_(module.bias)

    def _create_causal_mask(self, size: int) -> torch.Tensor:
        """Create causal attention mask."""
        mask = torch.triu(torch.ones(size, size), diagonal=1)
        mask = mask.masked_fill(mask == 1, float('-inf'))
        return mask

    def forward(
        self, 
        input_ids: torch.Tensor,
        state: torch.Tensor,
        attention_mask: Optional[torch.Tensor] = None,
        labels: Optional[torch.Tensor] = None
    ) -> Dict[str, torch.Tensor]:
        """
        Forward pass.

        Args:
            input_ids: Input token IDs (B, seq_len)
            state: Abstract state from diffusion predictor (B, state_dim)
            attention_mask: Padding mask (B, seq_len)
            labels: Target token IDs for training (B, seq_len)

        Returns:
            Dict with logits and optionally loss
        """
        B, L = input_ids.shape
        device = input_ids.device

        # Token + positional embeddings
        positions = torch.arange(L, device=device).unsqueeze(0).expand(B, -1)
        x = self.token_embed(input_ids) + self.pos_embed(positions)
        x = self.dropout(x)

        # Project state for cross-attention
        state_proj = self.state_proj(state).unsqueeze(1)  # (B, 1, embed_dim)

        # Causal mask
        causal_mask = self.causal_mask[:L, :L]

        # Apply decoder layers
        for layer in self.layers:
            x = layer(x, state_proj, causal_mask, attention_mask)

        x = self.norm(x)
        logits = self.lm_head(x)

        outputs = {'logits': logits}

        # Compute loss if labels provided
        if labels is not None:
            shift_logits = logits[..., :-1, :].contiguous()
            shift_labels = labels[..., 1:].contiguous()

            loss_fct = nn.CrossEntropyLoss(ignore_index=self.pad_token_id)
            loss = loss_fct(
                shift_logits.view(-1, self.vocab_size),
                shift_labels.view(-1)
            )
            outputs['loss'] = loss

        return outputs

    @torch.no_grad()
    def generate(
        self,
        state: torch.Tensor,
        prompt_ids: Optional[torch.Tensor] = None,
        max_length: int = 100,
        temperature: float = 1.0,
        top_k: int = 50,
        top_p: float = 0.95,
        do_sample: bool = True,
        num_return_sequences: int = 1,
        eos_token_id: int = 2
    ) -> torch.Tensor:
        """
        Generate text conditioned on state.

        Args:
            state: Abstract state (B, state_dim)
            prompt_ids: Optional prompt tokens (B, prompt_len)
            max_length: Maximum generation length
            temperature: Sampling temperature
            top_k: Top-k sampling
            top_p: Nucleus sampling
            do_sample: Whether to sample or use greedy decoding
            num_return_sequences: Number of sequences to generate
            eos_token_id: End-of-sequence token ID

        Returns:
            Generated token IDs
        """
        batch_size = state.shape[0]
        device = state.device

        # Expand for multiple sequences
        if num_return_sequences > 1:
            state = state.unsqueeze(1).repeat(1, num_return_sequences, 1)
            state = state.view(-1, self.state_dim)
            batch_size = batch_size * num_return_sequences

        # Initialize with prompt or BOS
        if prompt_ids is not None:
            if num_return_sequences > 1:
                prompt_ids = prompt_ids.unsqueeze(1).repeat(1, num_return_sequences, 1)
                prompt_ids = prompt_ids.view(-1, prompt_ids.shape[-1])
            input_ids = prompt_ids
        else:
            bos_token_id = 1  # Assuming BOS token
            input_ids = torch.full((batch_size, 1), bos_token_id, device=device, dtype=torch.long)

        finished = torch.zeros(batch_size, dtype=torch.bool, device=device)

        for _ in range(max_length):
            # Forward pass
            outputs = self.forward(input_ids, state)
            logits = outputs['logits']

            # Get logits for last position
            next_token_logits = logits[:, -1, :] / temperature

            # Apply top-k filtering
            if top_k > 0:
                indices_to_remove = next_token_logits < torch.topk(next_token_logits, top_k)[0][..., -1, None]
                next_token_logits[indices_to_remove] = float('-inf')

            # Apply top-p (nucleus) filtering
            if top_p < 1.0:
                sorted_logits, sorted_indices = torch.sort(next_token_logits, descending=True)
                cumulative_probs = torch.cumsum(F.softmax(sorted_logits, dim=-1), dim=-1)
                sorted_indices_to_remove = cumulative_probs > top_p
                sorted_indices_to_remove[..., 1:] = sorted_indices_to_remove[..., :-1].clone()
                sorted_indices_to_remove[..., 0] = 0

                indices_to_remove = sorted_indices_to_remove.scatter(1, sorted_indices, sorted_indices_to_remove)
                next_token_logits[indices_to_remove] = float('-inf')

            # Sample or greedy
            if do_sample:
                probs = F.softmax(next_token_logits, dim=-1)
                next_token = torch.multinomial(probs, num_samples=1)
            else:
                next_token = torch.argmax(next_token_logits, dim=-1, keepdim=True)

            # Append to sequence
            input_ids = torch.cat([input_ids, next_token], dim=1)

            # Check for EOS
            finished = finished | (next_token.squeeze(-1) == eos_token_id)
            if finished.all():
                break

        # Reshape if multiple sequences
        if num_return_sequences > 1:
            original_batch = state.shape[0] // num_return_sequences
            input_ids = input_ids.view(original_batch, num_return_sequences, -1)

        return input_ids

    def compute_perplexity(self, input_ids: torch.Tensor, 
                          state: torch.Tensor,
                          attention_mask: Optional[torch.Tensor] = None) -> float:
        """Compute perplexity on a sequence."""
        outputs = self.forward(input_ids, state, attention_mask, labels=input_ids)
        loss = outputs['loss']
        return torch.exp(loss).item()
