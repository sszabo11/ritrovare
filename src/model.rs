use anyhow::{Result, anyhow};
use ollama_rs::{
    Ollama,
    error::OllamaError,
    generation::{
        chat::{ChatMessage, ChatMessageResponse, MessageRole, request::ChatMessageRequest},
        embeddings::request::{EmbeddingsInput, GenerateEmbeddingsRequest},
    },
};
use tokio_stream::StreamExt;
use url::Url;

use crate::{browser::Tab, screen::SearchResult};

pub struct Model {
    model_name: String,
    ollama: Ollama,
}

const SYSTEM_PROMPT: &str = "You are Ritrovare, a personal browsing history assistant. The user has given you a list of pages they've previously read, with titles, URLs, visit counts, and timestamps. Answer their question using only this history — don't use outside knowledge. Be concise, reference specific pages by title when relevant, and if nothing in the history is relevant, say so plainly. Respond in markdown format.";

fn build_system_prompt() -> String {
    format!("/no_think\n\n{}", SYSTEM_PROMPT)
}

impl Model {
    pub fn new(model_name: &str) -> Self {
        Self {
            ollama: Ollama::default(),
            model_name: model_name.to_string(),
        }
    }

    pub async fn search(
        &self,
        query: &str,
        prev_messages: &[ChatMessage],
        on_token: impl AsyncFn(String),
    ) -> Result<SearchResult> {
        let mut all_messages: Vec<ChatMessage> =
            vec![ChatMessage::new(MessageRole::System, build_system_prompt())];
        all_messages.extend_from_slice(prev_messages);
        all_messages.push(ChatMessage::new(MessageRole::User, query.to_string()));

        log::info!("msgs: {:?}", all_messages);
        let req = ChatMessageRequest::new(self.model_name.clone(), all_messages).think(false);

        //let response = self.send_message(req).await?;

        let mut stream = self.ollama.send_chat_messages_stream(req).await?;
        let mut full_response = String::new();

        while let Some(res) = stream.next().await {
            match res {
                Ok(data) => {
                    full_response.push_str(&data.message.content);
                    on_token(data.message.content).await;
                }
                Err(_err) => {
                    log::info!("Failed to read stream token");
                    return Err(anyhow!("Failed to read stream token"));
                }
            }
        }

        let result = SearchResult {
            content: full_response,
        };
        Ok(result)
    }

    pub async fn warmup(&self) -> Result<()> {
        let request = ChatMessageRequest::new(
            self.model_name.clone(),
            vec![ChatMessage::user("hi".to_string())],
        );
        self.ollama.send_chat_messages(request).await?;
        Ok(())
    }

    //pub async fn send_message(
    //    &self,
    //    request: ChatMessageRequest,
    //) -> Result<ChatMessageResponse, OllamaError> {
    //    match self.ollama.send_chat_messages_stream(request).await {
    //        Ok(data) => Ok(data),
    //        Err(err) => {
    //            log::info!("Error: {}", err);
    //            Err(err)
    //        }
    //    }
    //}

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
