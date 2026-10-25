use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

impl Role {
    pub fn title(&self) -> &'static str {
        match self {
            Self::User => "You",
            Self::Assistant => "Assistant",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Default)]
pub struct Chat {
    pub messages: Vec<Message>,
    pub pending: bool,
    pub error: Option<String>,
}

impl Chat {
    pub fn begin(&mut self, prompt: &str) -> bool {
        let prompt = prompt.trim();
        if self.pending || prompt.is_empty() {
            return false;
        }
        self.messages.push(Message {
            role: Role::User,
            content: prompt.to_owned(),
        });
        self.pending = true;
        self.error = None;
        true
    }

    pub fn complete(&mut self, result: Result<String, String>) {
        self.pending = false;
        match result {
            Ok(content) => self.messages.push(Message {
                role: Role::Assistant,
                content,
            }),
            Err(error) => {
                // Failed prompts are not included in the next provider request.
                self.messages.pop();
                self.error = Some(error);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_and_concurrent_requests() {
        let mut chat = Chat::default();
        assert!(!chat.begin("  "));
        assert!(chat.begin(" Hello "));
        assert!(!chat.begin("Duplicate"));
        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].content, "Hello");
        chat.complete(Ok("Hi".into()));
        assert!(!chat.pending);
        assert!(matches!(chat.messages[1].role, Role::Assistant));
    }

    #[test]
    fn failure_allows_retry_without_a_dangling_user_turn() {
        let mut chat = Chat::default();
        chat.begin("Hello");
        chat.complete(Err("Provider unavailable".into()));
        assert!(!chat.pending);
        assert!(chat.messages.is_empty());
        assert!(chat.error.is_some());
        assert!(chat.begin("Retry"));
        assert!(chat.error.is_none());
    }
}
