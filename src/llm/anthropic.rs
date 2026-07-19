use super::provider::{
    LlmProvider, ProviderError, ProviderRequest, ProviderResponse, ProviderUsage,
    post_json_with_retry,
};
use futures_util::future::BoxFuture;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{Value, json};

pub(crate) struct AnthropicProvider {
    client: reqwest::Client,
    endpoint: String,
    model: String,
    headers: HeaderMap,
}

impl AnthropicProvider {
    pub fn new(
        client: reqwest::Client,
        endpoint: &str,
        model: &str,
        api_key: &str,
    ) -> Result<Self, ProviderError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(api_key)
                .map_err(|_| ProviderError::new("configuration", "llm.apiKey 包含无效字符"))?,
        );
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        Ok(Self {
            client,
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            headers,
        })
    }
}

impl LlmProvider for AnthropicProvider {
    fn rewrite<'a>(
        &'a self,
        request: &'a ProviderRequest,
    ) -> BoxFuture<'a, Result<ProviderResponse, ProviderError>> {
        Box::pin(async move {
            let body = json!({
                "model": self.model,
                "system": request.system_prompt,
                "messages": [{"role": "user", "content": request.user_prompt}],
                "max_tokens": request.max_tokens
            });
            let response = post_json_with_retry(
                &self.client,
                "anthropic",
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
                .get("content")
                .and_then(Value::as_array)
                .map(|parts| {
                    parts
                        .iter()
                        .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
                        .filter_map(|part| part.get("text").and_then(Value::as_str))
                        .collect::<String>()
                })
                .filter(|text| !text.is_empty())
                .ok_or_else(|| {
                    ProviderError::new("invalid_response", "Anthropic 响应缺少文本 content")
                        .with_request_id(request_id.clone())
                })?;
            Ok(ProviderResponse {
                text: content,
                request_id,
                finish_reason: response
                    .value
                    .get("stop_reason")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                usage: ProviderUsage {
                    input_tokens: response
                        .value
                        .pointer("/usage/input_tokens")
                        .and_then(Value::as_u64),
                    output_tokens: response
                        .value
                        .pointer("/usage/output_tokens")
                        .and_then(Value::as_u64),
                },
            })
        })
    }
}
