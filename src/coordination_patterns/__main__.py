"""CLI for multi-agent coordination patterns.

Usage:
    coordination-patterns extract "Find the Q1 sales report"
    coordination-patterns extract "Find the Q1 sales report" --model-llm qwen3.5:0.8b
    coordination-patterns extract "Find the Q1 sales report" --cache
    coordination-patterns --help
"""

from __future__ import annotations

import argparse
import sys

from coordination_patterns.llm_interface.config import EmbeddingConfig, LLMConfig
from coordination_patterns.intent_extractor.extractor import IntentExtractor


# Default values — used when the user provides no provider/model flags
DEFAULT_PROVIDER_LLM: str = "ollama-local"
DEFAULT_PROVIDER_EMBEDDING: str = "ollama-local"
DEFAULT_MODEL_LLM: str = "qwen3.5:2b"
DEFAULT_MODEL_EMBEDDING: str = "nomic-embed-text"
DEFAULT_HOST: str = "localhost"

# Available LLM providers
LLM_PROVIDER_OPTIONS: dict[str, dict[str, str]] = {
    "ollama-local": {
        "label": "ollama-local",
        "help": "Local Ollama instance (localhost:11434)",
    },
}

# Available embedding providers
EMBEDDING_PROVIDER_OPTIONS: dict[str, dict[str, str]] = {
    "ollama-local": {
        "label": "ollama-local",
        "help": "Local Ollama instance (localhost:11434)",
    },
}

# Available models
LLM_MODELS: dict[str, str] = {
    "qwen3.5:2b": "Qwen 3.5 2B — default",
    "qwen3.5:0.8b": "Qwen 3.5 0.8B — small/fast (testing)",
}

# Available embedding models
EMBEDDING_MODELS: dict[str, str] = {
    "nomic-embed-text": "Nomic Embed Text — semantic cache",
}


def build_llm_config(
    provider: str | None, model: str | None, host: str | None
) -> LLMConfig:
    """Build LLMConfig from CLI args, applying defaults."""
    p = provider or DEFAULT_PROVIDER_LLM
    m = model or DEFAULT_MODEL_LLM
    h = host or DEFAULT_HOST

    if p == "ollama-local":
        cfg = LLMConfig.ollama(host=h, model=m)
    else:
        print(f"Error: Unknown LLM provider '{p}'.", file=sys.stderr)
        print(
            f"Available: {', '.join(LLM_PROVIDER_OPTIONS.keys())}",
            file=sys.stderr,
        )
        sys.exit(1)

    return cfg


def build_embedding_config(
    provider: str | None, model: str | None, host: str | None
) -> EmbeddingConfig:
    """Build EmbeddingConfig from CLI args, applying defaults."""
    p = provider or DEFAULT_PROVIDER_EMBEDDING
    m = model or DEFAULT_MODEL_EMBEDDING
    h = host or DEFAULT_HOST

    if p == "ollama-local":
        cfg = EmbeddingConfig.ollama(host=h, model=m)
    else:
        print(
            f"Error: Unknown embedding provider '{p}'.",
            file=sys.stderr,
        )
        print(
            f"Available: {', '.join(EMBEDDING_PROVIDER_OPTIONS.keys())}",
            file=sys.stderr,
        )
        sys.exit(1)

    return cfg


def cmd_extract(args: argparse.Namespace) -> None:
    """Run the semantic intent extractor on user input."""
    llm_config = build_llm_config(args.provider_llm, args.model_llm, args.host)
    embed_config = build_embedding_config(
        args.provider_embedding, args.model_embedding, args.host
    )

    effective_llm_provider = args.provider_llm or DEFAULT_PROVIDER_LLM
    effective_embed_provider = args.provider_embedding or DEFAULT_PROVIDER_EMBEDDING

    print(f"LLM Provider: {effective_llm_provider} (model: {llm_config.model})")
    print(f"Endpoint: {llm_config.base_url}")
    print(f"Embed Provider: {effective_embed_provider} (model: {embed_config.model})")
    print(f"Embed Endpoint: {embed_config.base_url}")
    print(f"Cache: {'enabled' if args.cache else 'disabled'}")
    print(f"Input: {args.text}")
    print("---")

    try:
        with IntentExtractor(
            llm_config,
            embed_config=embed_config if args.cache else None,
            cache_enabled=args.cache,
        ) as extractor:
            result = extractor.process(args.text)
            print(f"\nResult: {result}")
    except Exception as e:
        print(f"\nError: {e}", file=sys.stderr)
        sys.exit(1)


def main() -> None:
    llm_list = "\n".join(
        f"  {name:15s} {info['help']}"
        for name, info in LLM_PROVIDER_OPTIONS.items()
    )
    embed_list = "\n".join(
        f"  {name:15s} {info['help']}"
        for name, info in EMBEDDING_PROVIDER_OPTIONS.items()
    )
    model_list = "\n".join(
        f"  {name:20s} {info}" for name, info in LLM_MODELS.items()
    )
    embed_model_list = "\n".join(
        f"  {name:20s} {info}" for name, info in EMBEDDING_MODELS.items()
    )

    epilog_text = (
        f"LLM providers:\n{llm_list}\n\n"
        f"Embedding providers:\n{embed_list}\n\n"
        f"LLM models:\n{model_list}\n\n"
        f"Embedding models:\n{embed_model_list}\n\n"
        f"Defaults: --provider-llm={DEFAULT_PROVIDER_LLM} --model-llm={DEFAULT_MODEL_LLM} "
        f"--provider-embedding={DEFAULT_PROVIDER_EMBEDDING} --model-embedding={DEFAULT_MODEL_EMBEDDING}\n\n"
        "Examples:\n"
        '  coordination-patterns extract "Find the Q1 sales report"\n'
        '  coordination-patterns extract "Find the Q1 sales report" --model-llm qwen3.5:0.8b\n'
        '  coordination-patterns extract "Find the Q1 sales report" --cache\n'
        '  coordination-patterns extract "Find the Q1 sales report" --cache --model-embedding nomic-embed-text\n'
    )

    parser = argparse.ArgumentParser(
        prog="coordination-patterns",
        description=(
            "Multi-Agent Coordination Patterns — "
            "semantic intent extraction and agent routing."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=epilog_text,
    )

    subparsers = parser.add_subparsers(dest="command", help="Available commands")

    # extract subcommand
    extract_parser = subparsers.add_parser(
        "extract",
        help="Extract semantic intent from natural language and route to agent",
        description=(
            "Run the full pipeline: Natural Language → LLM → "
            "RoutingIntent → AgentRouter → Agent"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )

    extract_parser.add_argument(
        "text",
        help="Natural language input text",
    )
    extract_parser.add_argument(
        "--provider-llm",
        default=None,
        help=f"LLM provider (default: {DEFAULT_PROVIDER_LLM})",
    )
    extract_parser.add_argument(
        "--provider-embedding",
        default=None,
        help=f"Embedding provider for semantic cache (default: {DEFAULT_PROVIDER_EMBEDDING})",
    )
    extract_parser.add_argument(
        "--model-llm",
        default=None,
        help=f"LLM model (default: {DEFAULT_MODEL_LLM})",
    )
    extract_parser.add_argument(
        "--model-embedding",
        default=None,
        help=f"Embedding model (default: {DEFAULT_MODEL_EMBEDDING})",
    )
    extract_parser.add_argument(
        "--host",
        default=None,
        help=f"Override host (default: {DEFAULT_HOST})",
    )
    extract_parser.add_argument(
        "--cache",
        action="store_true",
        default=False,
        help="Enable semantic intent cache (bypass LLM for similar queries)",
    )

    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        sys.exit(0)

    if args.command == "extract":
        cmd_extract(args)


if __name__ == "__main__":
    main()
