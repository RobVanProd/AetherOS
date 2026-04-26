"""
Scaled Model Configurations for Aurora and Aero.
Production-ready model sizes with optimized architectures.
"""
from typing import Dict, Any


class ModelConfig:
    """Base configuration class."""
    def __init__(self, **kwargs):
        for key, value in kwargs.items():
            setattr(self, key, value)

    def to_dict(self) -> Dict[str, Any]:
        return {k: v for k, v in self.__dict__.items() if not k.startswith('_')}


# Small Model (13M params) - Original
AURORA_SMALL = ModelConfig(
    name="Aurora-Small",
    state_dim=128,
    vocab_size=50000,
    jepa_vision_config={
        'image_size': 224,
        'patch_size': 16,
        'in_channels': 3,
        'embed_dim': 256,
        'num_layers': 2,
        'num_heads': 8
    },
    jepa_text_config={
        'vocab_size': 50000,
        'embed_dim': 256,
        'num_layers': 2,
        'num_heads': 8,
        'max_seq_len': 512
    },
    diffusion_hidden_dim=256,
    diffusion_num_layers=4,
    diffusion_num_timesteps=1000,
    diffusion_beta_schedule='cosine',
    llm_embed_dim=256,
    llm_num_layers=4,
    llm_num_heads=8,
    llm_max_seq_len=512,
    pad_token_id=0,
    use_flash_attention=False,
    use_mixed_precision=False,
    estimated_params=13_000_000
)

# Base Model (110M params)
AURORA_BASE = ModelConfig(
    name="Aurora-Base",
    state_dim=512,
    vocab_size=50000,
    jepa_vision_config={
        'image_size': 224,
        'patch_size': 16,
        'in_channels': 3,
        'embed_dim': 768,
        'num_layers': 6,
        'num_heads': 12
    },
    jepa_text_config={
        'vocab_size': 50000,
        'embed_dim': 768,
        'num_layers': 6,
        'num_heads': 12,
        'max_seq_len': 1024
    },
    diffusion_hidden_dim=1024,
    diffusion_num_layers=8,
    diffusion_num_timesteps=1000,
    diffusion_beta_schedule='cosine',
    llm_embed_dim=768,
    llm_num_layers=8,
    llm_num_heads=12,
    llm_max_seq_len=1024,
    pad_token_id=0,
    use_flash_attention=True,
    use_mixed_precision=True,
    estimated_params=110_000_000
)

# Large Model (400M params)
AURORA_LARGE = ModelConfig(
    name="Aurora-Large",
    state_dim=768,
    vocab_size=50000,
    jepa_vision_config={
        'image_size': 224,
        'patch_size': 16,
        'in_channels': 3,
        'embed_dim': 1024,
        'num_layers': 12,
        'num_heads': 16
    },
    jepa_text_config={
        'vocab_size': 50000,
        'embed_dim': 1024,
        'num_layers': 12,
        'num_heads': 16,
        'max_seq_len': 2048
    },
    diffusion_hidden_dim=1536,
    diffusion_num_layers=12,
    diffusion_num_timesteps=1000,
    diffusion_beta_schedule='cosine',
    llm_embed_dim=1024,
    llm_num_layers=12,
    llm_num_heads=16,
    llm_max_seq_len=2048,
    pad_token_id=0,
    use_flash_attention=True,
    use_mixed_precision=True,
    estimated_params=400_000_000
)

# XL Model (1B params)
AURORA_XL = ModelConfig(
    name="Aurora-XL",
    state_dim=1024,
    vocab_size=100000,
    jepa_vision_config={
        'image_size': 224,
        'patch_size': 14,
        'in_channels': 3,
        'embed_dim': 1280,
        'num_layers': 16,
        'num_heads': 16
    },
    jepa_text_config={
        'vocab_size': 100000,
        'embed_dim': 1280,
        'num_layers': 16,
        'num_heads': 16,
        'max_seq_len': 4096
    },
    diffusion_hidden_dim=2048,
    diffusion_num_layers=16,
    diffusion_num_timesteps=1000,
    diffusion_beta_schedule='cosine',
    llm_embed_dim=1280,
    llm_num_layers=16,
    llm_num_heads=16,
    llm_max_seq_len=4096,
    pad_token_id=0,
    use_flash_attention=True,
    use_mixed_precision=True,
    use_gradient_checkpointing=True,
    estimated_params=1_000_000_000
)

# Specialized: Code-World Model (for code/logic prediction)
AERO_CODE = ModelConfig(
    name="Aero-Code",
    state_dim=512,
    vocab_size=100000,  # Larger vocab for code tokens
    jepa_vision_config={
        'image_size': 224,
        'patch_size': 16,
        'in_channels': 3,
        'embed_dim': 768,
        'num_layers': 6,
        'num_heads': 12
    },
    jepa_text_config={
        'vocab_size': 100000,
        'embed_dim': 768,
        'num_layers': 8,  # Deeper for code understanding
        'num_heads': 12,
        'max_seq_len': 2048  # Longer for code context
    },
    diffusion_hidden_dim=1024,
    diffusion_num_layers=10,
    diffusion_num_timesteps=1000,
    diffusion_beta_schedule='cosine',
    llm_embed_dim=768,
    llm_num_layers=10,
    llm_num_heads=12,
    llm_max_seq_len=2048,
    pad_token_id=0,
    use_flash_attention=True,
    use_mixed_precision=True,
    # Code-specific settings
    special_tokens={
        'code_start': 50000,
        'code_end': 50001,
        'indent': 50002,
        'dedent': 50003,
        'newline': 50004
    },
    estimated_params=150_000_000
)

# Specialized: Long-Context Model (for extended reasoning)
AERO_LONG = ModelConfig(
    name="Aero-Long",
    state_dim=512,
    vocab_size=50000,
    jepa_vision_config={
        'image_size': 224,
        'patch_size': 16,
        'in_channels': 3,
        'embed_dim': 768,
        'num_layers': 6,
        'num_heads': 12
    },
    jepa_text_config={
        'vocab_size': 50000,
        'embed_dim': 768,
        'num_layers': 6,
        'num_heads': 12,
        'max_seq_len': 8192  # Very long context
    },
    diffusion_hidden_dim=1024,
    diffusion_num_layers=8,
    diffusion_num_timesteps=1000,
    diffusion_beta_schedule='cosine',
    llm_embed_dim=768,
    llm_num_layers=8,
    llm_num_heads=12,
    llm_max_seq_len=8192,
    pad_token_id=0,
    use_flash_attention=True,
    use_mixed_precision=True,
    use_memory_efficient_attention=True,
    attention_chunk_size=1024,
    estimated_params=130_000_000
)


def get_config(name: str) -> ModelConfig:
    """Get a model configuration by name."""
    configs = {
        'small': AURORA_SMALL,
        'base': AURORA_BASE,
        'large': AURORA_LARGE,
        'xl': AURORA_XL,
        'code': AERO_CODE,
        'long': AERO_LONG,
    }

    name_lower = name.lower()
    if name_lower not in configs:
        raise ValueError(f"Unknown config: {name}. Available: {list(configs.keys())}")

    return configs[name_lower]


def list_configs():
    """List all available configurations."""
    configs = {
        'small': AURORA_SMALL,
        'base': AURORA_BASE,
        'large': AURORA_LARGE,
        'xl': AURORA_XL,
        'code': AERO_CODE,
        'long': AERO_LONG,
    }

    print("Available Model Configurations:")
    print("-" * 60)
    for key, config in configs.items():
        params_m = config.estimated_params / 1_000_000
        print(f"  {key:8} | {config.name:15} | {params_m:6.0f}M params")

    return configs
