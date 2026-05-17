use crate::capability_router::{ActionType, AgentRouter, ResourceType, RoutingIntent};
use crate::llm_interface::{EmbeddingClient, LLMClient, LLMConfig, EmbeddingConfig};
use crate::semantic_cache::SemanticCache;

pub struct IntentExtractor {
    client: LLMClient,
    embed_client: Option<EmbeddingClient>,
    router: AgentRouter,
    pub cache: Option<SemanticCache>,
    cache_enabled: bool,
}

impl IntentExtractor {
    pub fn new(
        config: Option<LLMConfig>,
        embed_config: Option<EmbeddingConfig>,
        cache_enabled: bool,
        cache_store: &str,
        cache_store_path: Option<&str>,
    ) -> Self {
        let client = LLMClient::new(config);
        let embed_client = if cache_enabled {
            Some(EmbeddingClient::new(embed_config))
        } else {
            None
        };
        let cache = if cache_enabled {
            Some(SemanticCache::new(0.92, 1000, cache_store, cache_store_path))
        } else {
            None
        };

        Self {
            client,
            embed_client,
            router: AgentRouter::new(),
            cache,
            cache_enabled,
        }
    }

    pub fn extract(&mut self, user_input: &str) -> Result<RoutingIntent, String> {
        if self.cache_enabled {
            if let (Some(cache), Some(embed_client)) = (&mut self.cache, &self.embed_client) {
                let embedding = embed_client.embed(user_input)?;
                if let Some(cached) = cache.lookup(&embedding) {
                    println!("Cache HIT (threshold {})", cache.threshold());
                    return Ok(cached);
                }
            }
        }

        let intent_schema = Self::build_intent_schema();
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": format!("Extract the intent from this request:\n\n{user_input}")
        })];

        let system_prompt = r#"You are an intent extraction assistant.

Given a natural language request, extract:
- action: one of find, analyze, create
- resource: one of sales_report, server_log, document
- parameters: any relevant details as a dict

If the request doesn't match any action/resource combination,
still extract the closest action and resource you can infer."#;

        let result: serde_json::Value = match self.client.structured_chat(
            messages.clone(),
            intent_schema,
            Some(system_prompt),
        ) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Structured output failed ({e}), trying plain text fallback...");
                let raw = self.client.chat(messages, Some(system_prompt), None)?;
                serde_json::from_str(&raw).map_err(|e| format!("Fallback parse failed: {e}"))?
            }
        };

        let action: ActionType = serde_json::from_value(result["action"].clone())
            .map_err(|e| format!("Invalid action: {e}"))?;
        let resource: ResourceType = serde_json::from_value(result["resource"].clone())
            .map_err(|e| format!("Invalid resource: {e}"))?;
        let parameters = result.get("parameters").cloned().unwrap_or(serde_json::json!({}));

        let intent = RoutingIntent {
            action,
            resource,
            parameters,
        };

        if self.cache_enabled {
            if let (Some(cache), Some(embed_client)) = (&mut self.cache, &self.embed_client) {
                let embedding = embed_client.embed(user_input)?;
                cache.store(user_input, &embedding, &intent);
                println!("Cache MISS → stored for future hits");
            }
        }

        Ok(intent)
    }

    pub fn process(&mut self, user_input: &str) -> Result<String, String> {
        let intent = self.extract(user_input)?;
        println!(
            "Extracted intent: action={}, resource={}, params={}",
            serde_json::to_value(&intent.action).unwrap(),
            serde_json::to_value(&intent.resource).unwrap(),
            intent.parameters
        );
        Ok(self.router.route_request(&intent))
    }

    fn build_intent_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["find", "analyze", "create"],
                    "description": "The action to perform."
                },
                "resource": {
                    "type": "string",
                    "enum": ["sales_report", "server_log", "document"],
                    "description": "The resource to act on."
                },
                "parameters": {
                    "type": "object",
                    "description": "Additional parameters extracted from the request."
                }
            },
            "required": ["action", "resource"]
        })
    }
}
