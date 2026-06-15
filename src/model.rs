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

const SYSTEM_PROMPT: &str = "You are Ritrovare, a personal browsing history assistant. The user has given you a list of pages they've previously read, with titles, URLs, visit counts, and timestamps. Answer their question using only this history — don't use outside knowledge. Be concise, reference specific pages by title when relevant, and if nothing in the history is relevant, say so plainly. Respond in markdown format.";

impl Model {
    pub fn new(name: &str) -> Self {
        Self {
            ollama: Ollama::default(),
            name: name.to_string(),
        }
    }

    pub async fn search(&self, query: &str) -> Result<SearchResult> {
        let req = ChatMessageRequest::new(
            "gemma4".to_string(),
            vec![
                ChatMessage::new(MessageRole::System, SYSTEM_PROMPT.to_string()),
                ChatMessage::new(MessageRole::User, query.to_string()),
            ],
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
                log::info!("Error: {}", err);
                Err(err)
            }
        }
    }

    pub async fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        let input = EmbeddingsInput::Single(query.to_string());

        let request = GenerateEmbeddingsRequest::new("embeddinggemma".to_string(), input);

        let res = self.ollama.generate_embeddings(request).await?;

        Ok(res.embeddings[0].clone())
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
