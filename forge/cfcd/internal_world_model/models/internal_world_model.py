"""
Internal World Model: Complete Architecture
Integrates JEPA Encoder, Diffusion Predictor, and LLM Decoder.
"""
import torch
import torch.nn as nn
from typing import Dict, Optional, Tuple, List, Any
import sys
import os

# Add parent directory to path
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from models.jepa_encoder import JEPAEncoder
from models.diffusion_predictor import DiffusionPredictor
from models.llm_decoder import LLMDecoder


class InternalWorldModel(nn.Module):
    """
    Complete Internal World Model Architecture.

    Pipeline:
    1. JEPA Encoder: sensory_data -> abstract_state
    2. Diffusion Predictor: current_state -> future_state (via denoising)
    3. LLM Decoder: future_state -> language/code
    """
    def __init__(
        self,
        state_dim: int = 512,
        vocab_size: int = 50000,
        # JEPA Encoder config
        jepa_vision_config: Optional[Dict] = None,
        jepa_text_config: Optional[Dict] = None,
        jepa_multimodal: bool = True,
        # Diffusion Predictor config
        diffusion_hidden_dim: int = 1024,
        diffusion_num_layers: int = 8,
        diffusion_num_timesteps: int = 1000,
        diffusion_beta_schedule: str = 'cosine',
        # LLM Decoder config
        llm_embed_dim: int = 768,
        llm_num_layers: int = 12,
        llm_num_heads: int = 12,
        llm_max_seq_len: int = 512,
        pad_token_id: int = 0
    ):
        super().__init__()

        self.state_dim = state_dim
        self.vocab_size = vocab_size

        # 1. JEPA Encoder
        self.encoder = JEPAEncoder(
            state_dim=state_dim,
            vision_config=jepa_vision_config,
            text_config=jepa_text_config,
            multimodal=jepa_multimodal
        )

        # 2. Diffusion Predictor
        self.predictor = DiffusionPredictor(
            state_dim=state_dim,
            hidden_dim=diffusion_hidden_dim,
            num_layers=diffusion_num_layers,
            num_timesteps=diffusion_num_timesteps,
            beta_schedule=diffusion_beta_schedule
        )

        # 3. LLM Decoder
        self.decoder = LLMDecoder(
            vocab_size=vocab_size,
            state_dim=state_dim,
            embed_dim=llm_embed_dim,
            num_layers=llm_num_layers,
            num_heads=llm_num_heads,
            max_seq_len=llm_max_seq_len,
            pad_token_id=pad_token_id
        )

        self.pad_token_id = pad_token_id

    def encode(
        self,
        images: Optional[torch.Tensor] = None,
        text_tokens: Optional[torch.Tensor] = None,
        text_mask: Optional[torch.Tensor] = None
    ) -> Dict[str, torch.Tensor]:
        """
        Encode sensory input to abstract state.

        Returns:
            Dict with 'state' and optionally 'vision_state', 'text_state'
        """
        return self.encoder(images, text_tokens, text_mask)

    def predict(
        self,
        current_state: torch.Tensor,
        num_samples: int = 1,
        use_ddim: bool = True,
        ddim_timesteps: int = 50,
        return_trajectory: bool = False
    ) -> torch.Tensor:
        """
        Predict future state(s) from current state using diffusion.

        Args:
            current_state: Current abstract state (B, state_dim)
            num_samples: Number of future states to generate
            use_ddim: Use fast DDIM sampling
            ddim_timesteps: Number of DDIM steps
            return_trajectory: Return denoising trajectory

        Returns:
            Predicted future state(s)
        """
        if use_ddim:
            return self.predictor.ddim_sample(
                current_state, 
                num_samples=num_samples,
                ddim_timesteps=ddim_timesteps
            )
        else:
            return self.predictor.sample(
                current_state,
                num_samples=num_samples,
                return_trajectory=return_trajectory
            )

    def decode(
        self,
        future_state: torch.Tensor,
        prompt_ids: Optional[torch.Tensor] = None,
        max_length: int = 100,
        temperature: float = 1.0,
        top_k: int = 50,
        top_p: float = 0.95,
        do_sample: bool = True,
        num_return_sequences: int = 1
    ) -> torch.Tensor:
        """
        Decode future state to language/code.

        Args:
            future_state: Predicted future state (B, state_dim)
            prompt_ids: Optional prompt tokens
            max_length: Maximum generation length
            temperature: Sampling temperature
            top_k: Top-k sampling
            top_p: Nucleus sampling
            do_sample: Whether to sample
            num_return_sequences: Number of sequences per state

        Returns:
            Generated token IDs
        """
        return self.decoder.generate(
            future_state,
            prompt_ids=prompt_ids,
            max_length=max_length,
            temperature=temperature,
            top_k=top_k,
            top_p=top_p,
            do_sample=do_sample,
            num_return_sequences=num_return_sequences
        )

    def forward(
        self,
        # Encoder inputs
        images: Optional[torch.Tensor] = None,
        text_tokens: Optional[torch.Tensor] = None,
        text_mask: Optional[torch.Tensor] = None,
        # Training targets
        future_text_tokens: Optional[torch.Tensor] = None,
        future_text_mask: Optional[torch.Tensor] = None,
        # Generation parameters
        mode: str = 'train'  # 'train' or 'generate'
    ) -> Dict[str, Any]:
        """
        Full forward pass through the entire pipeline.

        Args:
            images: Input images (B, C, H, W)
            text_tokens: Input text tokens (B, seq_len)
            text_mask: Input text padding mask (B, seq_len)
            future_text_tokens: Target future text for training (B, seq_len)
            future_text_mask: Target future text mask (B, seq_len)
            mode: 'train' for training, 'generate' for inference

        Returns:
            Dict with outputs and losses
        """
        outputs = {}

        # Step 1: Encode current sensory input
        encoder_out = self.encode(images, text_tokens, text_mask)
        current_state = encoder_out['state']
        outputs['current_state'] = current_state
        outputs['encoder'] = encoder_out

        if mode == 'train':
            # Training mode: we need paired data (current_state, future_state, future_text)
            # For training, we assume future_text_tokens contains the "future" we want to predict

            # Encode future text to get target future state (for training predictor)
            with torch.no_grad():
                future_encoder_out = self.encode(text_tokens=future_text_tokens, text_mask=future_text_mask)
                target_future_state = future_encoder_out['state']

            # Step 2: Train diffusion predictor
            pred_noise, true_noise = self.predictor(target_future_state, current_state)
            diffusion_loss = nn.functional.mse_loss(pred_noise, true_noise)
            outputs['diffusion_loss'] = diffusion_loss

            # Step 3: Train decoder (conditioned on predicted future state)
            # Use the actual future state for teacher forcing during training
            decoder_out = self.decoder(
                input_ids=future_text_tokens,
                state=target_future_state,
                attention_mask=future_text_mask,
                labels=future_text_tokens
            )
            outputs['decoder_loss'] = decoder_out['loss']
            outputs['logits'] = decoder_out['logits']

            # Total loss
            outputs['loss'] = diffusion_loss + decoder_out['loss']

        else:  # generate mode
            # Step 2: Predict future state
            future_state = self.predict(current_state, num_samples=1, use_ddim=True)
            outputs['future_state'] = future_state

            # Step 3: Decode to language
            generated_ids = self.decode(future_state.squeeze(1))
            outputs['generated_ids'] = generated_ids

        return outputs

    def imagine(
        self,
        images: Optional[torch.Tensor] = None,
        text: Optional[str] = None,
        text_tokenizer = None,
        num_future_scenarios: int = 3,
        max_length: int = 100,
        temperature: float = 0.8
    ) -> List[Dict[str, Any]]:
        """
        Complete imagination pipeline: sensory_input -> future_scenarios.

        Args:
            images: Input image(s)
            text: Input text description
            text_tokenizer: Tokenizer for text
            num_future_scenarios: Number of future scenarios to generate
            max_length: Max generation length per scenario
            temperature: Sampling temperature

        Returns:
            List of scenario dicts with 'future_state' and 'description'
        """
        self.eval()

        # Encode current state
        text_tokens = None
        text_mask = None
        if text is not None and text_tokenizer is not None:
            encoded = text_tokenizer(text, return_tensors='pt', padding=True)
            text_tokens = encoded['input_ids'].to(next(self.parameters()).device)
            text_mask = encoded.get('attention_mask')

        with torch.no_grad():
            encoder_out = self.encode(images, text_tokens, text_mask)
            current_state = encoder_out['state']

            # Generate multiple future scenarios
            future_states = self.predict(
                current_state, 
                num_samples=num_future_scenarios,
                use_ddim=True
            )  # (B, num_scenarios, state_dim)

            scenarios = []
            for i in range(num_future_scenarios):
                state = future_states[:, i, :]
                generated = self.decode(
                    state,
                    max_length=max_length,
                    temperature=temperature,
                    num_return_sequences=1
                )

                scenarios.append({
                    'future_state': state,
                    'generated_ids': generated,
                    'scenario_index': i
                })

        return scenarios


class WorldModelTrainer:
    """Training utilities for the Internal World Model."""

    def __init__(
        self,
        model: InternalWorldModel,
        optimizer: torch.optim.Optimizer,
        device: str = 'cuda' if torch.cuda.is_available() else 'cpu'
    ):
        self.model = model.to(device)
        self.optimizer = optimizer
        self.device = device

    def train_step(
        self,
        batch: Dict[str, torch.Tensor]
    ) -> Dict[str, float]:
        """Single training step."""
        self.model.train()

        # Move batch to device
        images = batch.get('images')
        if images is not None:
            images = images.to(self.device)

        text_tokens = batch.get('text_tokens')
        if text_tokens is not None:
            text_tokens = text_tokens.to(self.device)

        text_mask = batch.get('text_mask')
        if text_mask is not None:
            text_mask = text_mask.to(self.device)

        future_text_tokens = batch['future_text_tokens'].to(self.device)
        future_text_mask = batch.get('future_text_mask')
        if future_text_mask is not None:
            future_text_mask = future_text_mask.to(self.device)

        # Forward pass
        outputs = self.model(
            images=images,
            text_tokens=text_tokens,
            text_mask=text_mask,
            future_text_tokens=future_text_tokens,
            future_text_mask=future_text_mask,
            mode='train'
        )

        loss = outputs['loss']

        # Backward pass
        self.optimizer.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(self.model.parameters(), 1.0)
        self.optimizer.step()

        return {
            'loss': loss.item(),
            'diffusion_loss': outputs['diffusion_loss'].item(),
            'decoder_loss': outputs['decoder_loss'].item()
        }

    def validate(
        self,
        dataloader: torch.utils.data.DataLoader
    ) -> Dict[str, float]:
        """Validation loop."""
        self.model.eval()
        total_loss = 0
        total_diffusion_loss = 0
        total_decoder_loss = 0
        num_batches = 0

        with torch.no_grad():
            for batch in dataloader:
                images = batch.get('images')
                if images is not None:
                    images = images.to(self.device)

                text_tokens = batch.get('text_tokens')
                if text_tokens is not None:
                    text_tokens = text_tokens.to(self.device)

                text_mask = batch.get('text_mask')
                if text_mask is not None:
                    text_mask = text_mask.to(self.device)

                future_text_tokens = batch['future_text_tokens'].to(self.device)
                future_text_mask = batch.get('future_text_mask')
                if future_text_mask is not None:
                    future_text_mask = future_text_mask.to(self.device)

                outputs = self.model(
                    images=images,
                    text_tokens=text_tokens,
                    text_mask=text_mask,
                    future_text_tokens=future_text_tokens,
                    future_text_mask=future_text_mask,
                    mode='train'
                )

                total_loss += outputs['loss'].item()
                total_diffusion_loss += outputs['diffusion_loss'].item()
                total_decoder_loss += outputs['decoder_loss'].item()
                num_batches += 1

        return {
            'loss': total_loss / num_batches,
            'diffusion_loss': total_diffusion_loss / num_batches,
            'decoder_loss': total_decoder_loss / num_batches
        }
