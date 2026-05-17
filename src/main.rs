use clap::{Parser, Subcommand};
use local_agent::intent_extractor::IntentExtractor;
use local_agent::llm_interface::{EmbeddingConfig, LLMConfig};

const DEFAULT_PROVIDER_LLM: &str = "ollama-local";
const DEFAULT_PROVIDER_EMBEDDING: &str = "ollama-local";
const DEFAULT_MODEL_LLM: &str = "qwen3.5:2b";
const DEFAULT_MODEL_EMBEDDING: &str = "nomic-embed-text";
const DEFAULT_HOST: &str = "localhost";
const DEFAULT_CACHE_STORE: &str = "sqlite";

#[derive(Parser)]
#[command(
    name = "local-agent",
    about = "Local AI agent coordination — semantic intent extraction and agent routing.",
    long_about = None,
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract semantic intent from natural language and route to agent
    #[command(
        about = "Run the full pipeline: Natural Language → LLM → RoutingIntent → AgentRouter → Agent"
    )]
    Extract {
        /// Natural language input text
        text: String,

        /// LLM provider (default: ollama-local)
        #[arg(long = "provider-llm")]
        provider_llm: Option<String>,

        /// Embedding provider for semantic cache (default: ollama-local)
        #[arg(long = "provider-embedding")]
        provider_embedding: Option<String>,

        /// LLM model (default: qwen3.5:2b)
        #[arg(long = "model-llm")]
        model_llm: Option<String>,

        /// Embedding model (default: nomic-embed-text)
        #[arg(long = "model-embedding")]
        model_embedding: Option<String>,

        /// Override host (default: localhost)
        #[arg(long)]
        host: Option<String>,

        /// Enable semantic intent cache (bypass LLM for similar queries)
        #[arg(long, default_value_t = false)]
        cache: bool,

        /// Cache persistence backend (default: memory)
        #[arg(long = "cache-store", default_value = DEFAULT_CACHE_STORE, value_parser = ["memory", "sqlite"])]
        cache_store: String,

        /// Path to SQLite cache database
        #[arg(long = "cache-path")]
        cache_path: Option<String>,
    },
}

fn build_llm_config(
    provider: Option<String>,
    model: Option<String>,
    host: Option<String>,
) -> LLMConfig {
    let p = provider.unwrap_or_else(|| DEFAULT_PROVIDER_LLM.into());
    let m = model.unwrap_or_else(|| DEFAULT_MODEL_LLM.into());
    let h = host.unwrap_or_else(|| DEFAULT_HOST.into());

    match p.as_str() {
        "ollama-local" => LLMConfig::ollama(&h, &m, false),
        _ => {
            eprintln!("Error: Unknown LLM provider '{p}'.");
            eprintln!("Available: ollama-local");
            std::process::exit(1);
        }
    }
}

fn build_embedding_config(
    provider: Option<String>,
    model: Option<String>,
    host: Option<String>,
) -> EmbeddingConfig {
    let p = provider.unwrap_or_else(|| DEFAULT_PROVIDER_EMBEDDING.into());
    let m = model.unwrap_or_else(|| DEFAULT_MODEL_EMBEDDING.into());
    let h = host.unwrap_or_else(|| DEFAULT_HOST.into());

    match p.as_str() {
        "ollama-local" => EmbeddingConfig::ollama(&h, &m),
        _ => {
            eprintln!("Error: Unknown embedding provider '{p}'.");
            eprintln!("Available: ollama-local");
            std::process::exit(1);
        }
    }
}

fn cmd_extract(args: &Commands) {
    let Commands::Extract {
        text,
        provider_llm,
        provider_embedding,
        model_llm,
        model_embedding,
        host,
        cache,
        cache_store,
        cache_path,
    } = args;

    let llm_config = build_llm_config(provider_llm.clone(), model_llm.clone(), host.clone());
    let embed_config = build_embedding_config(
        provider_embedding.clone(),
        model_embedding.clone(),
        host.clone(),
    );

    let effective_llm_provider = provider_llm.as_deref().unwrap_or(DEFAULT_PROVIDER_LLM);
    let effective_embed_provider = provider_embedding
        .as_deref()
        .unwrap_or(DEFAULT_PROVIDER_EMBEDDING);

    println!(
        "LLM Provider: {} (model: {})",
        effective_llm_provider, llm_config.model
    );
    println!("Endpoint: {}", llm_config.base_url);
    println!(
        "Embed Provider: {} (model: {})",
        effective_embed_provider, embed_config.model
    );
    println!("Embed Endpoint: {}", embed_config.base_url);
    println!("Cache: {}", if *cache { "enabled" } else { "disabled" });
    if *cache {
        println!("Cache Store: {}", cache_store);
        let default_cache_path = {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            format!("{}/.local/share/local-agent/cache.db", home)
        };
        let cp = cache_path.as_deref().unwrap_or(&default_cache_path);
        println!("Cache Path: {}", cp);
    }
    println!("Input: {}", text);
    println!("---");

    let llm_client = local_agent::llm_interface::LLMClient::new(Some(llm_config));
    let embed_client = if *cache {
        Some(local_agent::llm_interface::EmbeddingClient::new(Some(
            embed_config,
        )))
    } else {
        None
    };

    let mut extractor = IntentExtractor::new(
        llm_client,
        embed_client,
        *cache,
        cache_store,
        if cache_store == "sqlite" {
            cache_path.as_deref()
        } else {
            None
        },
    );

    match extractor.process(text) {
        Ok(result) => println!("\nResult: {}", result),
        Err(e) => {
            eprintln!("\nError: {}", e);
            std::process::exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Extract { .. } => cmd_extract(&cli.command),
    }
}
