use super::provider::{
    LlmProvider, ProviderError, ProviderRequest, ProviderResponse, ProviderUsage,
    post_json_with_retry,
};
use futures_util::future::BoxFuture;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde_json::{Value, json};

pub(crate) struct OpenAiCompatibleProvider {
    client: reqwest::Client,
    provider: String,
    endpoint: String,
    model: String,
    headers: HeaderMap,
}

impl OpenAiCompatibleProvider {
    pub fn new(
        client: reqwest::Client,
        provider: &str,
        endpoint: &str,
        model: &str,
        api_key: &str,
    ) -> Result<Self, ProviderError> {
        let mut headers = HeaderMap::new();
        let authorization = HeaderValue::from_str(&format!("Bearer {api_key}"))
            .map_err(|_| ProviderError::new("configuration", "llm.apiKey 包含无效字符"))?;
        headers.insert(AUTHORIZATION, authorization);
        if provider == "openrouter" {
            headers.insert(
                "http-referer",
                HeaderValue::from_static("https://github.com/day253/typeless-ibus"),
            );
            headers.insert(
                "x-openrouter-title",
                HeaderValue::from_static("typeless-ibus"),
            );
        }
        Ok(Self {
            client,
            provider: provider.to_string(),
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            headers,
        })
    }
}

impl LlmProvider for OpenAiCompatibleProvider {
    fn rewrite<'a>(
        &'a self,
        request: &'a ProviderRequest,
    ) -> BoxFuture<'a, Result<ProviderResponse, ProviderError>> {
        Box::pin(async move {
            let mut body = json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": request.system_prompt},
                    {"role": "user", "content": request.user_prompt}
                ],
                "stream": false
            });
            if self.provider == "deepseek" {
                body["thinking"] = json!({"type": "disabled"});
            }
            let response = post_json_with_retry(
                &self.client,
                &self.provider,
                &self.endpoint,
                &self.headers,
                &body,
            )
            .await?;
            let request_id = response.request_id.clone().or_else(|| {
                response
                    .value
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            });
            let content = response
                .value
                .pointer("/choices/0/message/content")
                .and_then(extract_text_content)
                .ok_or_else(|| {
                    ProviderError::new(
                        "invalid_response",
                        "LLM 响应缺少 choices[0].message.content",
                    )
                    .with_request_id(request_id.clone())
                })?;
            Ok(ProviderResponse {
                text: content,
                request_id,
                finish_reason: response
                    .value
                    .pointer("/choices/0/finish_reason")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                usage: ProviderUsage {
                    input_tokens: response
                        .value
                        .pointer("/usage/prompt_tokens")
                        .and_then(Value::as_u64),
                    output_tokens: response
                        .value
                        .pointer("/usage/completion_tokens")
                        .and_then(Value::as_u64),
                },
            })
        })
    }
}

fn extract_text_content(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let text = parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<String>();
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    }
}
