use anyhow::{anyhow, Result};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    ai::ask_mia_tools::{dispatch_tool, tool_schema, AskMiaToolContext, AskMiaToolName},
    api::investigation::{
        clean_sentence, normalize_ask_mia_evidence, parse_json_payload, AskMiaAnswer,
        AskMiaRunContext, ContractIntelligence, InternalEvidence, MarketIntelligence,
    },
    config::Config,
};

const ASK_MIA_MAX_TOOL_ROUNDS: usize = 3;

#[derive(Debug)]
pub struct AskMiaFunctionCallingResult {
    pub provider: String,
    pub answer: AskMiaAnswer,
    pub tool_trace: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ToolingChatRequest<'a> {
    model: &'a str,
    messages: Vec<Value>,
    tools: Vec<Value>,
    tool_choice: &'static str,
    temperature: f64,
    max_tokens: u32,
    reasoning: ReasoningConfig,
}

#[derive(Debug, Serialize)]
struct ReasoningConfig {
    effort: &'static str,
}

#[derive(Debug, Deserialize)]
struct ToolingChatResponse {
    model: String,
    choices: Vec<ToolingChoice>,
}

#[derive(Debug, Deserialize)]
struct ToolingChoice {
    message: ToolingMessage,
}

#[derive(Debug, Deserialize)]
struct ToolingMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ToolingToolCall>>,
}

#[derive(Debug, Deserialize)]
struct ToolingToolCall {
    id: String,
    function: ToolingFunctionCall,
}

#[derive(Debug, Deserialize)]
struct ToolingFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct AskMiaPayload {
    short_answer: String,
    why: String,
    evidence: Vec<String>,
    next_move: String,
}

pub async fn run_function_calling(
    config: &Config,
    question: &str,
    internal: &InternalEvidence,
    contract_intelligence: &ContractIntelligence,
    market_intelligence: &MarketIntelligence,
    run_context: Option<&AskMiaRunContext>,
) -> Result<AskMiaFunctionCallingResult> {
    if config.llm_models.is_empty() {
        return Err(anyhow!("LLM model list is empty"));
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let system = "You are Ask MIA v2, a function-calling token copilot for Four.Meme launches on BNB Chain. Decide which internal read-only tools are required before answering. Use only the tool outputs you receive. If attached run context is present, treat it as the continuity layer for the current investigation and use it when explaining what changed, why a run escalated, or what is still being monitored. Do not invent facts, identities, or sources. Keep the final answer plain, professional, and action-oriented. Once you have enough evidence, return strict JSON only with keys short_answer, why, evidence, next_move. evidence must be an array of 2 to 5 short bullet strings.";
    let user_content = if let Some(run_context) = run_context {
        format!(
            "User question: {question}\n\nAttached run context:\n{}",
            render_run_context_for_prompt(run_context)
        )
    } else {
        question.to_string()
    };

    let mut messages = vec![
        json!({"role":"system","content": system}),
        json!({"role":"user","content": user_content}),
    ];
    let tools = tool_schema();
    let context = AskMiaToolContext {
        internal,
        contract_intelligence,
        market_intelligence,
    };
    let mut tool_trace = Vec::new();

    for _ in 0..ASK_MIA_MAX_TOOL_ROUNDS {
        let response = call_openai_tooling(
            &client,
            config,
            ToolingChatRequest {
                model: config.llm_models.first().expect("validated above"),
                messages: messages.clone(),
                tools: tools.clone(),
                tool_choice: "auto",
                temperature: 0.1,
                max_tokens: 500,
                reasoning: ReasoningConfig { effort: "high" },
            },
        )
        .await?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Tooling response returned no choices"))?;

        if let Some(tool_calls) = choice.message.tool_calls {
            let assistant_message = json!({
                "role": "assistant",
                "content": choice.message.content.unwrap_or_default(),
                "tool_calls": tool_calls.iter().map(|tool_call| {
                    json!({
                        "id": tool_call.id,
                        "type": "function",
                        "function": {
                            "name": tool_call.function.name,
                            "arguments": tool_call.function.arguments,
                        }
                    })
                }).collect::<Vec<_>>()
            });
            messages.push(assistant_message);

            for tool_call in tool_calls {
                if !tool_call.function.arguments.trim().is_empty()
                    && tool_call.function.arguments.trim() != "{}"
                {
                    let _: Value = serde_json::from_str(&tool_call.function.arguments)?;
                }

                let tool_name =
                    AskMiaToolName::parse(&tool_call.function.name).ok_or_else(|| {
                        anyhow!("Unsupported Ask MIA tool: {}", tool_call.function.name)
                    })?;
                let tool_result = dispatch_tool(tool_name, &context);
                let tool_name = tool_name.as_str().to_string();
                if !tool_trace.iter().any(|existing| existing == &tool_name) {
                    tool_trace.push(tool_name);
                }
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_call.id,
                    "content": serde_json::to_string(&tool_result)?,
                }));
            }

            continue;
        }

        let payload: AskMiaPayload =
            parse_json_payload(&choice.message.content.unwrap_or_default())?;
        return Ok(AskMiaFunctionCallingResult {
            provider: response.model,
            answer: AskMiaAnswer {
                short_answer: clean_sentence(
                    &payload.short_answer,
                    "MIA needs a little more evidence before making a stronger call.",
                ),
                why: clean_sentence(
                    &payload.why,
                    "The current answer is based on the internal tools Ask MIA selected.",
                ),
                evidence: normalize_ask_mia_evidence(payload.evidence, &[]),
                next_move: clean_sentence(
                    &payload.next_move,
                    "Stay disciplined and wait for the next clear signal.",
                ),
            },
            tool_trace,
        });
    }

    Err(anyhow!(
        "Ask MIA function calling exhausted its tool rounds"
    ))
}

fn render_run_context_for_prompt(run_context: &AskMiaRunContext) -> String {
    let recent_events = run_context
        .recent_events
        .iter()
        .map(|event| format!("- {}: {}", event.label, event.detail))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Run ID: {}\nStatus: {}\nCurrent stage: {}\nContinuity note: {}\nLatest reason: {}\nLatest evidence delta: {}\nRecent events:\n{}",
        run_context.run_id,
        run_context.status,
        run_context.current_stage,
        run_context.continuity_note,
        run_context.latest_reason.as_deref().unwrap_or("n/a"),
        run_context.latest_evidence_delta.as_deref().unwrap_or("n/a"),
        if recent_events.is_empty() { "- none".to_string() } else { recent_events }
    )
}

async fn call_openai_tooling(
    client: &reqwest::Client,
    config: &Config,
    request: ToolingChatRequest<'_>,
) -> Result<ToolingChatResponse> {
    for model in &config.llm_models {
        let response = client
            .post(format!(
                "{}/v1/chat/completions",
                config.llm_api_url.trim_end_matches('/')
            ))
            .bearer_auth(&config.llm_api_key)
            .json(&ToolingChatRequest {
                model,
                messages: request.messages.clone(),
                tools: request.tools.clone(),
                tool_choice: request.tool_choice,
                temperature: request.temperature,
                max_tokens: request.max_tokens,
                reasoning: ReasoningConfig { effort: "high" },
            })
            .send()
            .await?;

        let status = response.status();
        if status == StatusCode::TOO_MANY_REQUESTS {
            tracing::warn!(model = %model, "Ask MIA v2 was rate limited, rolling to next model");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Ask MIA v2 tool call request failed with {} on {}: {}",
                status,
                model,
                body
            ));
        }

        return Ok(response.json::<ToolingChatResponse>().await?);
    }

    Err(anyhow!(
        "Ask MIA v2 was rate limited across all configured models"
    ))
}
