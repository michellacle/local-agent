"""CLI for multi-agent coordination patterns.

Usage:
    coordination-patterns extract "Find the Q1 sales report" --provider-llm ollama-local
    coordination-patterns extract "Find the Q1 sales report" --provider-llm ollama-local --provider-embedding ollama-local --cache
    coordination-patterns --help
"""

from __future__ import annotations

import argparse
import sys

from coordination_patterns.llm_interface.config import EmbeddingConfig, LLMConfig
from coordination_patterns.intent_extractor.extractor import IntentExtractor


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
    provider: str, model: str | None, host: str | None
) -> LLMConfig:
    """Build LLMConfig from CLI args."""
    if provider == "ollama-local":
        cfg = LLMConfig.ollama(host=host or "localhost")
    else:
        print(f"Error: Unknown LLM provider '{provider}'.", file=sys.stderr)
        print(
            f"Available: {', '.join(LLM_PROVIDER_OPTIONS.keys())}",
            file=sys.stderr,
        )
        sys.exit(1)

    if model:
        cfg.model = model
    return cfg


def build_embedding_config(
    provider: str, model: str | None, host: str | None
) -> EmbeddingConfig:
    """Build EmbeddingConfig from CLI args."""
    if provider == "ollama-local":
        cfg = EmbeddingConfig.ollama(host=host or "localhost")
    else:
        print(
            f"Error: Unknown embedding provider '{provider}'.",
            file=sys.stderr,
        )
        print(
            f"Available: {', '.join(EMBEDDING_PROVIDER_OPTIONS.keys())}",
            file=sys.stderr,
        )
        sys.exit(1)

    if model:
        cfg.model = model
    return cfg


def cmd_extract(args: argparse.Namespace) -> None:
    """Run the semantic intent extractor on user input."""
    llm_config = build_llm_config(args.provider_llm, args.model, args.host)

    embed_config = None
    if args.cache:
        if not args.provider_embedding:
            print(
                "Error: --cache requires --provider-embedding.",
                file=sys.stderr,
            )
            sys.exit(1)
        embed_config = build_embedding_config(
            args.provider_embedding, args.embed_model, args.host
        )

    print(f"LLM Provider: {args.provider_llm} (model: {llm_config.model})")
    print(f"Endpoint: {llm_config.base_url}")
    if embed_config:
        print(f"Embed Provider: {args.provider_embedding} (model: {embed_config.model})")
        print(f"Embed Endpoint: {embed_config.base_url}")
    print(f"Cache: {'enabled' if args.cache else 'disabled'}")
    print(f"Input: {args.text}")
    print("---")

    try:
        with IntentExtractor(
            llm_config,
            embed_config=embed_config,
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

    parser = argparse.ArgumentParser(
        prog="coordination-patterns",
        description=(
            "Multi-Agent Coordination Patterns — "
            "semantic intent extraction and agent routing."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            f"LLM providers:\n{llm_list}\n\n"
            f"Embedding providers:\n{embed_list}\n\n"
            f"LLM models:\n{model_list}\n\n"
            f"Embedding models:\n{embed_model_list}\n\n"
            "Examples:\n"
            '  coordination-patterns extract "Find the Q1 sales report" --provider-llm ollama-local\n'
            '  coordination-patterns extract "Find the Q1 sales report" --provider-llm ollama-local --provider-embedding ollama-local --cache\n'
        ),
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
        choices=list(LLM_PROVIDER_OPTIONS.keys()),
        required=True,
        help="LLM provider for intent extraction (required)",
    )
    extract_parser.add_argument(
        "--provider-embedding",
        choices=list(EMBEDDING_PROVIDER_OPTIONS.keys()),
        default=None,
        help="Embedding provider for semantic cache (required with --cache)",
    )
    extract_parser.add_argument(
        "--model",
        default=None,
        help="LLM model (default: qwen3.5:2b)",
    )
    extract_parser.add_argument(
        "--embed-model",
        default=None,
        help="Embedding model (default: nomic-embed-text)",
    )
    extract_parser.add_argument(
        "--host",
        default=None,
        help="Override host (default: localhost)",
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
