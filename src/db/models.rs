use dotenv::dotenv;
use rusqlite::{Result as SqliteResult, Row};
use serde::{Deserialize, Serialize};

/// Represents a file record in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: Option<i64>,
    pub filename: String,
    pub file_id: String,
    pub message_id: String,
    pub url: String,
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub uuid: String,
    pub custom_url: String,
    pub upload_time: Option<String>,
}

impl FileRecord {
    /// Create a new FileRecord instance
    pub fn new(
        filename: String,
        url: String,
        year: u32,
        month: u32,
        day: u32,
        uuid: String,
        file_id: String,
        message_id: String,
    ) -> Self {
        dotenv().ok();
        let protocol = dotenv::var("PROTOCOL").unwrap_or("http".to_string());
        let custom_domain = dotenv::var("CUSTOM_DOMAIN").unwrap_or("localhost".to_string());
        let custom_port = dotenv::var("CUSTOM_PORT").unwrap_or_else(|_| {
            if protocol == "http" {
                "80".to_string()
            } else if protocol == "https" {
                "443".to_string()
            } else {
                panic!("Unsupported protocol in environment variables")
            }
        });
        let custom_url = format!(
            "{}://{}:{}/{}/{}/{}/{}",
            protocol, custom_domain, custom_port, year, month, day, uuid
        );
        Self {
            id: None,
            filename,
            file_id,
            message_id,
            url,
            year,
            month,
            day,
            uuid,
            custom_url: custom_url.clone(),
            upload_time: None,
        }
    }

    /// Convert from SQLite Row to FileRecord
    pub fn from_row(row: &Row) -> SqliteResult<Self> {
        Ok(Self {
            id: Some(row.get("id")?),
            filename: row.get("filename")?,
            file_id: row.get("file_id")?,
            message_id: row.get("message_id")?,
            url: row.get("url")?,
            year: row.get("year")?,
            month: row.get("month")?,
            day: row.get("day")?,
            uuid: row.get("uuid")?,
            custom_url: row.get("custom_url")?,
            upload_time: row.get("upload_time")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_record_creation() {
        let file_record = FileRecord::new(
            "test.txt".to_string(),
            "http://example.com/test.txt".to_string(),
            2023,
            10,
            1,
            "uuid-1234".to_string(),
            "file-id-1234".to_string(),
            "message-id-1234".to_string(),
        );

        println!("{:#?}", file_record);
    }
}
