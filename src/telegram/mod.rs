use api::ResSendDocument;
pub mod api;

#[allow(async_fn_in_trait)]
#[allow(dead_code)]
pub trait Bot {
    async fn get_updates(&self) -> Result<String, Box<dyn std::error::Error>>;
    async fn send_document(
        &self,
        document: Vec<u8>,
        document_name: &str,
    ) -> Result<ResSendDocument, Box<dyn std::error::Error>>;
    async fn get_file_url(&self, file_id: &str) -> Result<String, Box<dyn std::error::Error>>;
    async fn delete_message(&self, message_id: String) -> Result<bool, Box<dyn std::error::Error>>;
}
