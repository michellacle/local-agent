"""CLI for multi-agent coordination patterns.

Usage:
    coordination-patterns extract "Find the Q1 sales report"
    coordination-patterns extract "Analyze the server logs" --llm ollama
    coordination-patterns extract "Find the sales report" --llm ollama --model llama3.2
    coordination-patterns --help
"""

from __future__ import annotations

import argparse
import sys

from coordination_patterns.llm_interface.config import LLMConfig
from coordination_patterns.intent_extractor.extractor import IntentExtractor


# Available LLM backends
LLM_OPTIONS = {
    "cacique": {
        "label": "cacique",
        "help": "Cacique server (papia.tailde85bf.ts.net:8880) — default",
    },
    "ollama": {
        "label": "ollama",
        "help": "Local Ollama instance (localhost:11434)",
    },
    "openai": {
        "label": "openai",
        "help": "OpenAI API (requires OPENAI_API_KEY env var)",
    },
}

DEFAULT_MODEL = {
    "cacique": "kokoro",
    "ollama": "llama3.2",
    "openai": "gpt-4o",
}


def build_config(llm: str, model: str | None, host: str | None) -> LLMConfig:
    """Build LLMConfig from CLI args."""
    if llm == "cacique":
        cfg = LLMConfig.cacique()
    elif llm == "ollama":
        host = host or "localhost"
        cfg = LLMConfig.ollama(host=host)
    elif llm == "openai":
        cfg = LLMConfig.openai()
    else:
        print(f"Error: Unknown LLM backend '{llm}'.", file=sys.stderr)
        print(f"Available: {', '.join(LLM_OPTIONS.keys())}", file=sys.stderr)
        sys.exit(1)

    if model:
        cfg.model = model
    return cfg


def cmd_extract(args: argparse.Namespace) -> None:
    """Run the semantic intent extractor on user input."""
    config = build_config(args.llm, args.model, args.host)

    print(f"LLM: {args.llm} (model: {config.model})")
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
    backend_list = "\n".join(
        f"  {name:10s} {info['help']}" for name, info in LLM_OPTIONS.items()
    )

    parser = argparse.ArgumentParser(
        prog="coordination-patterns",
        description=(
            "Multi-Agent Coordination Patterns — "
            "semantic intent extraction and agent routing."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            f"Available LLM backends:\n{backend_list}\n\n"
            "Examples:\n"
            "  coordination-patterns extract \"Find the Q1 sales report\"\n"
            "  coordination-patterns extract \"Analyze server logs\" --llm ollama\n"
            "  coordination-patterns extract \"Find sales report\" --llm ollama --model llama3.2\n"
        ),
    )

    subparsers = parser.add_subparsers(dest="command", help="Available commands")

    # extract subcommand
    backend_list = "\n".join(
        f"  {name:10s} {info['help']}" for name, info in LLM_OPTIONS.items()
    )

    extract_parser = subparsers.add_parser(
        "extract",
        help="Extract semantic intent from natural language and route to agent",
        description=(
            "Run the full pipeline: Natural Language → LLM → "
            "RoutingIntent → AgentRouter → Agent"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=f"Available LLM backends:\n{backend_list}",
    )

    extract_parser.add_argument(
        "text",
        help="Natural language input text",
    )
    extract_parser.add_argument(
        "--llm",
        choices=list(LLM_OPTIONS.keys()),
        default="cacique",
        help="LLM backend to use (default: cacique)",
    )
    extract_parser.add_argument(
        "--model",
        default=None,
        help="Override the model name (default depends on --llm)",
    )
    extract_parser.add_argument(
        "--host",
        default=None,
        help="Override host (used with --llm ollama or cacique)",
    )

    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        sys.exit(0)

    if args.command == "extract":
        cmd_extract(args)


if __name__ == "__main__":
    main()
