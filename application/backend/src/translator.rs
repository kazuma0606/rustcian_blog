use reqwest::Client;
use rustacian_blog_core::BlogError;

use crate::config::AppConfig;

/// Azure Translator Text API v3 adapter.
/// Translates Japanese content to English.
/// For HTML input, pass `text_type = "html"` to preserve markup.
#[derive(Debug, Clone)]
pub struct AzureTranslatorAdapter {
    client: Client,
    endpoint: String,
    api_key: String,
}

impl AzureTranslatorAdapter {
    pub fn new(endpoint: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            api_key,
        }
    }

    /// Translate HTML content from Japanese to English, preserving markup.
    pub async fn translate_html(&self, html: &str) -> Result<String, BlogError> {
        self.translate(html, "html").await
    }

    /// Translate plain text from Japanese to English.
    pub async fn translate_text(&self, text: &str) -> Result<String, BlogError> {
        self.translate(text, "plain").await
    }

    async fn translate(&self, content: &str, text_type: &str) -> Result<String, BlogError> {
        let type_param = if text_type == "html" {
            "&textType=html"
        } else {
            ""
        };
        let url = format!(
            "{}/translate?api-version=3.0&from=ja&to=en{type_param}",
            self.endpoint.trim_end_matches('/')
        );

        let response = self
            .client
            .post(&url)
            .header("Ocp-Apim-Subscription-Key", &self.api_key)
            .json(&serde_json::json!([{"Text": content}]))
            .send()
            .await
            .map_err(|e| BlogError::Storage(format!("translator request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BlogError::Storage(format!(
                "translator api ({status}): {body}"
            )));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| BlogError::Parse(e.to_string()))?;

        result
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|item| item["translations"].as_array())
            .and_then(|translations| translations.first())
            .and_then(|t| t["text"].as_str())
            .ok_or_else(|| BlogError::Parse("translator: unexpected response shape".to_owned()))
            .map(|s| s.to_owned())
    }
}

pub fn build_translator(config: &AppConfig) -> Option<AzureTranslatorAdapter> {
    let endpoint = config.azure_translator_endpoint.clone()?;
    let api_key = config.azure_translator_api_key.clone()?;
    Some(AzureTranslatorAdapter::new(endpoint, api_key))
}
