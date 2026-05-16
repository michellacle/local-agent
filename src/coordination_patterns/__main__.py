"""CLI for multi-agent coordination patterns.

Usage:
    coordination-patterns extract "Find the Q1 sales report"
    coordination-patterns extract "Analyze the server logs" --model qwen3.5:2b
    coordination-patterns extract "Find the sales report" --host minadioro --model qwen2.5
    coordination-patterns --help
"""

from __future__ import annotations

import argparse
import sys

from coordination_patterns.llm_interface.config import LLMConfig
from coordination_patterns.intent_extractor.extractor import IntentExtractor


def build_config(model: str | None, host: str | None) -> LLMConfig:
    """Build LLMConfig from CLI args."""
    cfg = LLMConfig.ollama(host=host or "localhost")
    if model:
        cfg.model = model
    return cfg


def cmd_extract(args: argparse.Namespace) -> None:
    """Run the semantic intent extractor on user input."""
    config = build_config(args.model, args.host)

    print(f"Provider: ollama-local (model: {config.model}, host: {config.base_url})")
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
    parser = argparse.ArgumentParser(
        prog="coordination-patterns",
        description=(
            "Multi-Agent Coordination Patterns — "
            "semantic intent extraction and agent routing."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "Provider: ollama-local (OpenAI-compatible)\n\n"
            "Examples:\n"
            '  coordination-patterns extract "Find the Q1 sales report"\n'
            '  coordination-patterns extract "Analyze server logs" --model qwen3.5:2b\n'
            '  coordination-patterns extract "Find sales report" --host minadioro --model qwen3.5:2b\n'
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
        "--model",
        default=None,
        help="Override the model name (default: qwen3.5:2b)",
    )
    extract_parser.add_argument(
        "--host",
        default=None,
        help="Override Ollama host (default: localhost:11434)",
    )

    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        sys.exit(0)

    if args.command == "extract":
        cmd_extract(args)


if __name__ == "__main__":
    main()
