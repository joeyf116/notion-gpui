use crate::features::chat::Message;
use serde::Deserialize;
use std::time::Duration;

pub trait ChatProvider: Send + Sync {
    fn label(&self) -> &'static str;
    fn reply(&self, messages: &[Message]) -> Result<String, String>;
}

pub struct DemoProvider;

impl ChatProvider for DemoProvider {
    fn label(&self) -> &'static str {
        "Local demo · no AI model or network"
    }

    fn reply(&self, _: &[Message]) -> Result<String, String> {
        Ok(
            "Demo response (not AI): Break work into small tasks, assign an owner, and move cards \
            from Backlog to In progress to Done. Set OPENAI_API_KEY and OPENAI_MODEL before \
            launching to use a real AI provider."
                .into(),
        )
    }
}

pub struct OpenAiProvider {
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").ok()?;
        let model = std::env::var("OPENAI_MODEL").ok()?;
        if api_key.trim().is_empty() || model.trim().is_empty() {
            return None;
        }
        Some(Self { api_key, model })
    }
}

#[derive(Deserialize)]
struct Completion {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: CompletionMessage,
}

#[derive(Deserialize)]
struct CompletionMessage {
    content: Option<String>,
}

fn completion_text(completion: Completion) -> Result<String, String> {
    completion
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| "The AI provider returned no text.".into())
}

impl ChatProvider for OpenAiProvider {
    fn label(&self) -> &'static str {
        "OpenAI · sending shares this chat with OpenAI"
    }

    fn reply(&self, messages: &[Message]) -> Result<String, String> {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .max_redirects(0)
            .build();
        let agent = ureq::Agent::new_with_config(config);
        let mut response = agent
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", &["Bearer", &self.api_key].join(" "))
            .send_json(serde_json::json!({
                "model": self.model,
                "messages": messages,
                "max_completion_tokens": 2048,
                "store": false,
            }))
            .map_err(|_| {
                "AI request failed. Check your credentials, model, quota, and connection. \
                Your draft is available to retry."
                    .to_owned()
            })?;
        let completion = response
            .body_mut()
            .with_config()
            .limit(1024 * 1024)
            .read_json::<Completion>()
            .map_err(|_| "Unable to read the AI response.".to_owned())?;
        completion_text(completion)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_is_explicitly_not_an_ai_model() {
        let provider = DemoProvider;
        assert!(provider.label().contains("no AI"));
        assert!(provider.reply(&[]).unwrap().contains("not AI"));
    }

    #[test]
    fn handles_empty_and_valid_provider_responses() {
        for json in [
            r#"{"choices":[]}"#,
            r#"{"choices":[{"message":{"content":null}}]}"#,
            r#"{"choices":[{"message":{"content":" "}}]}"#,
        ] {
            assert!(completion_text(serde_json::from_str(json).unwrap()).is_err());
        }
        let response = r#"{"choices":[{"message":{"content":"Plan a release"}}]}"#;
        assert_eq!(
            completion_text(serde_json::from_str(response).unwrap()).unwrap(),
            "Plan a release"
        );
    }
}
