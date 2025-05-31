use super::Bot;
use log::{debug, error, info};
use reqwest;
use serde_json;

pub struct TelegramBot {
    api_url: String,
    base_url: String,
    token: String,
    chat_id: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ResSendDocument {
    pub file_id: String,
    pub file_name: String,
    pub file_url: String,
    pub message_id: String,
}

impl TelegramBot {
    pub fn new(token: &str, chat_id: &str) -> Self {
        let api_url = format!("https://api.telegram.org");
        TelegramBot {
            api_url: api_url.clone(),
            token: token.to_string(),
            chat_id: chat_id.to_string(),
            base_url: format!("{}/bot{}", api_url, token),
        }
    }
}

impl Bot for TelegramBot {
    async fn get_updates(&self) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/getUpdates", self.base_url);
        debug!("Requesting updates from: {}", url);

        let response = reqwest::get(&url).await?;
        if response.status().is_success() {
            let text = response.text().await?;
            info!("Received updates: {}", text);
            Ok(text)
        } else {
            let status = response.status();
            let error_text = response.text().await?;
            error!("Failed to get updates: {} - {}", status, error_text);
            Err(format!("Failed to get updates: {} - {}", status, error_text).into())
        }
    }

    async fn send_document(
        &self,
        file: Vec<u8>,
        file_name: &str,
    ) -> Result<ResSendDocument, Box<dyn std::error::Error>> {
        let url = format!("{}/sendDocument", self.base_url);
        debug!("Sending document (from file) to: {}", url);

        let part = reqwest::multipart::Part::bytes(file).file_name(file_name.to_string());
        let form = reqwest::multipart::Form::new()
            .part("document", part)
            .text("chat_id", self.chat_id.clone());

        let response = reqwest::Client::new()
            .post(&url)
            .multipart(form)
            .send()
            .await
            .unwrap();

        if response.status().is_success() {
            debug!("Document sent successfully: {}", response.status());
            let json: serde_json::Value = response
                .json()
                .await
                .expect("Failed to parse JSON response");
            let file_id: &serde_json::Value = json
                .get("result")
                .and_then(|r| r.get("document"))
                .and_then(|d| d.get("file_id"))
                .expect("Expected 'file_id' in document");
            debug!("File ID: {}", file_id);
            let message_id: &serde_json::Value = json
                .get("result")
                .and_then(|r| r.get("message_id"))
                .expect("Expected 'message_id' in document");
            debug!("Message ID: {}", message_id);
            // Check if there is a sticker file_id, and return "sticker_[file_id].webp" if present
            let sticker_file_id = json
                .get("result")
                .and_then(|r| r.get("sticker"))
                .and_then(|s| s.get("file_id"))
                .and_then(|v| v.as_str());

            let file_name = if let Some(sticker_id) = sticker_file_id {
                format!("sticker_{}.webp", sticker_id)
            } else {
                json.get("result")
                    .and_then(|r| r.get("document"))
                    .and_then(|d| d.get("file_name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string()
            };
            let file_url = self.get_file_url(file_id.as_str().unwrap()).await?;
            Ok(ResSendDocument {
                file_id: file_id.as_str().unwrap().to_string(),
                file_name,
                file_url,
                message_id: message_id.as_u64().unwrap().to_string(),
            })
        } else {
            let status = response.status();
            let error_json: serde_json::Value = response.json().await?;
            error!("Failed to send document: {} - {}", status, error_json);
            Err(format!("Failed to send document: {} - {}", status, error_json).into())
        }
    }

    async fn get_file_url(&self, file_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        let request_url = format!("{}/getFile?file_id={}", self.base_url, file_id);
        debug!("Requesting file path from: {}", request_url);
        let response = reqwest::get(&request_url)
            .await
            .expect("Failed to get file path");
        let json: serde_json::Value = response
            .json()
            .await
            .expect("Failed to parse JSON response");

        if json.get("ok").unwrap() == &serde_json::Value::Bool(false) {
            error!("Failed to get file path: {:?}", json);
            return Err(format!("Failed to get file path").into());
        }
        let file_path = json
            .get("result")
            .and_then(|r| r.get("file_path"))
            .and_then(|p| p.as_str())
            .expect("Expected 'file_path' in response");

        Ok(format!(
            "{}/file/bot{}/{}",
            self.api_url, self.token, file_path
        ))
    }
    async fn delete_message(&self, message_id: String) -> Result<bool, Box<dyn std::error::Error>> {
        let url = format!("{}/deleteMessage", self.base_url);
        let params = [
            ("chat_id", self.chat_id.as_str()),
            ("message_id", message_id.as_str()),
        ];
        let client = reqwest::Client::new();
        let response = client.post(&url).form(&params).send().await?;
        let json: serde_json::Value = response.json().await?;
        if !json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            return Err(format!("Delete message failed: {:?}", json).into());
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Read};

    use super::*;

    #[test]
    fn test_telegram_bot_new() {
        let bot = TelegramBot::new(
            "7280383975:AAFAGg3HkTVoiK0ttxwDbQ1FJPQNIWMQA60",
            "test_chat_id",
        );
        assert_eq!(bot.base_url, "https://api.telegram.org");
        assert_eq!(bot.token, "test_token");
        assert_eq!(bot.chat_id, "test_chat_id");
    }

    #[tokio::test]
    async fn test_telegram_bot_send_document() {
        let bot = TelegramBot::new(
            "7280383975:AAFAGg3HkTVoiK0ttxwDbQ1FJPQNIWMQA60",
            "5542167123",
        );
        let mut file = File::open("Cargo.toml").expect("Failed to open test file");
        let buffer: Vec<u8> = {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).expect("Failed to read file");
            buf
        };
        let result = bot.send_document(buffer, "Cargo.toml").await.unwrap();
        debug!("Document sent successfully: {:?}", result);
    }

    #[tokio::test]
    async fn test_telegram_bot_get_updates() {
        let bot = TelegramBot::new(
            "7280383975:AAFAGg3HkTVoiK0ttxwDbQ1FJPQNIWMQA60",
            "5542167123",
        );
        let result = bot.get_updates().await.unwrap();
        // assert!(result.is_empty(), "Failed to get updates: {:?}", result);
        println!("Updates: {}", result);
    }
    #[tokio::test]
    async fn test_telegram_bot_get_file_url() {
        let bot = TelegramBot::new(
            "7280383975:AAFAGg3HkTVoiK0ttxwDbQ1FJPQNIWMQA60",
            "5542167123",
        );
        let result = bot
            .get_file_url("BQACAgUAAxkDAAOuaDhXi1iDdlREj6kYPDELBgFRYoMAAhYWAAKSMsBV0h_fe79aBRc2BA")
            .await
            .unwrap();
        println!("File URL: {}", result);
    }

    #[tokio::test]
    async fn test_telegram_bot_delete_message() {
        let bot = TelegramBot::new(
            "7280383975:AAFAGg3HkTVoiK0ttxwDbQ1FJPQNIWMQA60",
            "5542167123",
        );
        let result = bot.delete_message("174".to_string()).await.unwrap();
        println!("Delete message result: {}", result);
    }
}
