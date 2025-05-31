use super::models::FileRecord;
use log::{debug, error, info};
use rusqlite::{Connection, Result};
use std::path::Path;

pub struct Database {
    db_path: String,
}

impl Database {
    pub fn new(db_path: &str) -> Self {
        let db = Database {
            db_path: db_path.to_string(),
        };
        if let Err(e) = db.init_db() {
            eprintln!("Failed to initialize database: {}", e);
        }
        db
    }

    pub fn init_db(&self) -> Result<(), Box<dyn std::error::Error>> {
        if Path::new(&self.db_path).exists() {
            info!("Database already exists.");
            println!("Database already exists.");
            return Ok(());
        }

        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                filename TEXT NOT NULL,
                file_id TEXT,
                message_id TEXT,
                url TEXT NOT NULL,
                year INTEGER,
                month INTEGER,
                day INTEGER,
                uuid TEXT NOT NULL,
                custom_url TEXT,
                upload_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        info!("Database initialized.");
        Ok(())
    }

    /// Insert a new file record using the FileRecord struct
    pub fn insert_file(&self, new_file: FileRecord) -> Result<i64> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT INTO files (filename, file_id, message_id, url, year, month, day, uuid, custom_url) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                new_file.filename,
                new_file.file_id,
                new_file.message_id,
                new_file.url,
                new_file.year,
                new_file.month,
                new_file.day,
                new_file.uuid,
                new_file.custom_url,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }
    pub fn get_all_records(&self) -> Result<Vec<FileRecord>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare("SELECT * FROM files")?;
        let rows = stmt.query_map([], |row| FileRecord::from_row(row))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.expect("Failed to map row to FileRecord"));
        }
        Ok(records)
    }

    pub fn get_file_record_by_id(&self, id: i64) -> Result<Option<FileRecord>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare("SELECT * FROM files WHERE id = ?1")?;
        let mut rows = stmt.query_map([id], |row| FileRecord::from_row(row))?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    fn get_record_by_data_and_uuid(
        &self,
        year: u32,
        month: u32,
        day: u32,
        uuid: &str,
    ) -> Result<Option<FileRecord>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT * FROM files WHERE uuid = ?1 AND year = ?2 AND month = ?3 AND day = ?4",
        )?;

        let mut rows = stmt.query_map(rusqlite::params![uuid, year, month, day], |row| {
            FileRecord::from_row(row)
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn del_record_by_id(&self, id: i64) -> Result<usize> {
        let conn = Connection::open(&self.db_path)?;
        let rows_affected = conn.execute("DELETE FROM files WHERE id = ?1", [id])?;
        if rows_affected == 0 {
            error!("No record found with id: {}", id);
        } else {
            info!("Deleted record with id: {}", id);
        }
        Ok(rows_affected)
    }

    /// Fetch the binary content of the file from the URL stored in the database record.
    pub async fn get_record_content(
        &self,
        year: u32,
        month: u32,
        day: u32,
        uuid: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let record = self.get_record_by_data_and_uuid(year, month, day, uuid)?;
        let file_record = record.ok_or("Record not found")?;
        let url = &file_record.url;
        info!("Fetching content from URL: {}", url);
        let response = reqwest::get(url).await.unwrap();
        info!("Response status code: {}", response.status());
        if !response.status().is_success() {
            return Err(format!("Failed to fetch content from URL: {}", url).into());
        }
        let content = response.bytes().await.unwrap().to_vec();
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_initialization() {
        let db = Database::new("test.db");
        assert!(db.init_db().is_ok());
        // Clean up test database file
        std::fs::remove_file("test.db").unwrap();
    }

    #[test]
    fn test_insert_file() {
        let db = Database::new("test_files.db");
        db.init_db().unwrap();

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

        let row_id = db.insert_file(file_record).unwrap();
        println!("Inserted row ID: {}", row_id);
        assert!(row_id > 0);

        // Clean up test database file
        // std::fs::remove_file("test_insert.db").unwrap();
    }

    #[test]
    fn test_get_all_records() {
        let db = Database::new("db.db");
        db.init_db().unwrap();
        let records = db.get_all_records().unwrap();
        assert!(!records.is_empty());
        println!("Retrieved records: {:#?}", records);

        // Clean up test database file
        // std::fs::remove_file("test_get_all.db").unwrap();
    }

    #[test]
    fn test_get_record_by_data_and_uuid() {
        let db = Database::new("db.db");
        db.init_db().unwrap();

        // let file_record = FileRecord::new(
        //     "test3.txt".to_string(),
        //     "http://example.com/test3.txt".to_string(),
        //     2023,
        //     10,
        //     3,
        //     "uuid-91011".to_string(),
        //     "file-id-91011".to_string(),
        //     "message-id-91011".to_string(),
        // );

        // db.insert_file(file_record).unwrap();
        let retrieved_record = db
            .get_record_by_data_and_uuid(2025, 5, 31, "6d477385-6336-4f4f-9b3f-0a45b17db477")
            .unwrap();
        assert!(retrieved_record.is_some());
        println!("Retrieved record: {:#?}", retrieved_record);

        // Clean up test database file
        // std::fs::remove_file("test_get_by_data.db").unwrap();
    }

    #[test]
    fn test_del_record_by_id() {
        let db = Database::new("db.db");
        db.init_db().unwrap();

        // let file_record = FileRecord::new(
        //     "test4.txt".to_string(),
        //     "http://example.com/test4.txt".to_string(),
        //     2023,
        //     10,
        //     4,
        //     "uuid-121314".to_string(),
        //     "file-id-121314".to_string(),
        //     "message-id-121314".to_string(),
        // );

        // let row_id = db.insert_file(file_record).unwrap();
        let row_id = 181;
        let rows_deleted = db.del_record_by_id(row_id).unwrap();
        // assert_eq!(rows_deleted, 1);
        println!("Deleted record with ID: {}", rows_deleted);

        // Clean up test database file
        // std::fs::remove_file("test_del_by_id.db").unwrap();
    }
}
