use anyhow::Result;
use ollama_rs::{
    Ollama,
    error::OllamaError,
    generation::{
        chat::{ChatMessage, ChatMessageResponse, MessageRole, request::ChatMessageRequest},
        embeddings::request::{EmbeddingsInput, GenerateEmbeddingsRequest},
    },
};
use url::Url;

use crate::{browser::Tab, screen::SearchResult};

pub struct Model {
    name: String,
    ollama: Ollama,
}

impl Model {
    pub fn new(name: &str) -> Self {
        Self {
            ollama: Ollama::default(),
            name: name.to_string(),
        }
    }

    pub async fn search(&self, query: String) -> Result<SearchResult> {
        let req = ChatMessageRequest::new(
            "gemma4".to_string(),
            vec![ChatMessage::new(MessageRole::User, query)],
        );

        let response = self.send_message(req).await?;

        let result = SearchResult {
            content: response.message.content,
        };
        Ok(result)
    }

    pub async fn send_message(
        &self,
        request: ChatMessageRequest,
    ) -> Result<ChatMessageResponse, OllamaError> {
        match self.ollama.send_chat_messages(request).await {
            Ok(data) => Ok(data),
            Err(err) => {
                eprintln!("Error: {}", err);
                Err(err)
            }
        }
    }

    pub async fn embed_tabs(&self, tabs: &Vec<Tab>) -> Result<Vec<Vec<f32>>> {
        let text: Vec<String> = tabs
            .iter()
            .map(|tab| format!("{} {}", tab.title, extract_domain(&tab.url)))
            .collect();

        let input = EmbeddingsInput::Multiple(text);

        let request = GenerateEmbeddingsRequest::new("embeddinggemma".to_string(), input);

        let res = self.ollama.generate_embeddings(request).await?;

        Ok(res.embeddings)
    }
}

fn extract_domain(url: &str) -> String {
    let parsed = Url::parse(url).expect("Failed to parse url");

    parsed
        .domain()
        .expect(format!("No domain in '{}'", url).as_str())
        .to_string()
}
