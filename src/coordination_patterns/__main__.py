"""CLI for multi-agent coordination patterns.

Usage:
    coordination-patterns extract "Find the Q1 sales report"
    coordination-patterns extract "Analyze the server logs" --provider ollama-local --model qwen3.5:2b
    coordination-patterns extract "Find the sales report" --provider ollama-local --host minadioro
    coordination-patterns --help
"""

from __future__ import annotations

import argparse
import sys

from coordination_patterns.llm_interface.config import LLMConfig
from coordination_patterns.intent_extractor.extractor import IntentExtractor


# Available providers
PROVIDER_OPTIONS = {
    "ollama-local": {
        "label": "ollama-local",
        "help": "Local Ollama instance (localhost:11434)",
    },
}


def build_config(provider: str, model: str | None, host: str | None) -> LLMConfig:
    """Build LLMConfig from CLI args."""
    if provider == "ollama-local":
        cfg = LLMConfig.ollama(host=host or "localhost")
    else:
        print(f"Error: Unknown provider '{provider}'.", file=sys.stderr)
        print(f"Available: {', '.join(PROVIDER_OPTIONS.keys())}", file=sys.stderr)
        sys.exit(1)

    if model:
        cfg.model = model
    return cfg


def cmd_extract(args: argparse.Namespace) -> None:
    """Run the semantic intent extractor on user input."""
    config = build_config(args.provider, args.model, args.host)

    print(f"Provider: {args.provider} (model: {config.model})")
    print(f"Endpoint: {config.base_url}")
    print(f"Input: {args.text}")
    print("---")

    try:
        with IntentExtractor(config) as extractor:
            result = extractor.process(args.text)
            print(f"\nResult: {result}")
    except Exception as e:
        print(f"\nError: {e}", file=sys.stderr)
        sys.exit(1)


def main():
    provider_list = "\n".join(
        f"  {name:15s} {info['help']}" for name, info in PROVIDER_OPTIONS.items()
    )

    parser = argparse.ArgumentParser(
        prog="coordination-patterns",
        description=(
            "Multi-Agent Coordination Patterns — "
            "semantic intent extraction and agent routing."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            f"Available providers:\n{provider_list}\n\n"
            "Examples:\n"
            '  coordination-patterns extract "Find the Q1 sales report"\n'
            '  coordination-patterns extract "Analyze server logs" --provider ollama-local --model qwen3.5:2b\n'
            '  coordination-patterns extract "Find sales report" --provider ollama-local --host minadioro\n'
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
        epilog=f"Available providers:\n{provider_list}",
    )

    extract_parser.add_argument(
        "text",
        help="Natural language input text",
    )
    extract_parser.add_argument(
        "--provider",
        choices=list(PROVIDER_OPTIONS.keys()),
        default="ollama-local",
        help="LLM provider to use (default: ollama-local)",
    )
    extract_parser.add_argument(
        "--model",
        default=None,
        help="Override the model name (default: qwen3.5:2b)",
    )
    extract_parser.add_argument(
        "--host",
        default=None,
        help="Override host (default: localhost)",
    )

    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        sys.exit(0)

    if args.command == "extract":
        cmd_extract(args)


if __name__ == "__main__":
    main()
