use actix_cors::Cors;
use actix_multipart::Multipart;
use actix_web::{App, HttpResponse, HttpServer, Responder, delete, get, post};
use db::FileRecord;
use dotenv::dotenv;
use futures_util::StreamExt as _;
use reqwest;
use std::env;
use telegram::Bot;
mod telegram;
use telegram::api::TelegramBot;
mod db;
use chrono::Datelike;
use db::db::Database;
use log::{debug, error, info, warn};

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[get("/getUpdates")]
async fn get_updates() -> impl Responder {
    let res_string = reqwest::get(
        "https://api.telegram.org/bot7280383975:AAFAGg3HkTVoiK0ttxwDbQ1FJPQNIWMQA60/getUpdates",
    )
    .await
    .unwrap()
    .text()
    .await
    .unwrap();
    HttpResponse::Ok().body(res_string)
}

#[post("/upload")]
async fn upload_file(mut payload: Multipart) -> impl Responder {
    while let Some(item) = payload.next().await {
        match item {
            Ok(mut field) => {
                let content_disposition = field.content_disposition();
                let filename = content_disposition
                    .and_then(|cd| cd.get_filename())
                    .map(|f| f.to_string())
                    .unwrap_or_else(|| "uploaded_file".to_string());

                // Save the file to memory
                let mut file_bytes = Vec::new();
                while let Some(chunk) = field.next().await {
                    let data = match chunk {
                        Ok(data) => data,
                        Err(e) => {
                            return HttpResponse::InternalServerError()
                                .body(format!("Cannot read file: {}", e));
                        }
                    };
                    file_bytes.extend_from_slice(&data);
                }

                let bot = TelegramBot::new(
                    &env::var("TG_BOT_TOKEN").expect("TG_BOT_TOKEN must be set"),
                    &env::var("TG_CHAT_ID").expect("TG_CHAT_ID must be set"),
                );
                // Send the file to Telegram
                match bot.send_document(file_bytes, &filename).await {
                    Ok(res) => {
                        let db = Database::new("db.db");
                        let now = chrono::Local::now();
                        let current_year: u32 = now.year() as u32;
                        let current_month: u32 = now.month();
                        let current_day: u32 = now.day();
                        let uuid = uuid::Uuid::new_v4().to_string();
                        db.init_db().unwrap();
                        if let Ok(row_id) = db.insert_file(FileRecord::new(
                            filename.clone(),
                            res.file_url.clone(),   // Placeholder URL
                            current_year,           // Example year
                            current_month,          // Example month
                            current_day,            // Example day
                            uuid,                   // Placeholder UUID
                            res.file_id.clone(),    // Use the file_id from the response
                            res.message_id.clone(), // Placeholder message ID
                        )) {
                            return HttpResponse::Ok().json(serde_json::json!({
                                "message": "File uploaded successfully",
                                "file_id": res.file_id,
                                "message_id": res.message_id,
                                "url": res.file_url,
                                "row_id": row_id,
                            }));
                        }
                    }
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(serde_json::json!({
                            "message": "Failed to send to Telegram",
                            "error": e.to_string()
                        }));
                    }
                }
            }
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Failed to parse field: {}", e));
            }
        }
    }
    HttpResponse::BadRequest().body("No file field received")
}

#[get("/files")]
async fn get_files() -> impl Responder {
    let db = Database::new("db.db");
    db.init_db().unwrap();
    match db.get_all_records() {
        Ok(records) => HttpResponse::Ok().json(records),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error fetching files: {}", e)),
    }
}

#[get("/find/{year}/{month}/{day}/{uuid}")]
async fn get_file(path: actix_web::web::Path<(u32, u32, u32, String)>) -> impl Responder {
    let (year, month, day, uuid) = path.into_inner();

    let db = Database::new("db.db");
    db.init_db().unwrap();

    match db.get_record_content(year, month, day, &uuid).await {
        Ok(content) => HttpResponse::Ok()
            .content_type("application/octet-stream")
            .append_header((
                "Content-Disposition",
                format!("attachment; filename={}", uuid),
            ))
            .body(content),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}

#[delete("/del/{file_id}")]
async fn delete_file(path: actix_web::web::Path<i64>) -> impl Responder {
    let file_id = path.into_inner();

    debug!("Try to delete file_id: {}", file_id);

    let db = Database::new("db.db");
    db.init_db().unwrap();

    match db.get_file_record_by_id(file_id) {
        Ok(Some(record)) => {
            debug!("DB record: {:?}", record);

            let tg_file_id = &record.file_id;
            let tg_message_id = &record.message_id;

            debug!(
                "tg_file_id: {}, tg_message_id: {}",
                tg_file_id, tg_message_id
            );

            let bot = TelegramBot::new(
                &env::var("TG_BOT_TOKEN").expect("TG_BOT_TOKEN must be set"),
                &env::var("TG_CHAT_ID").expect("TG_CHAT_ID must be set"),
            );

            let chat_id = env::var("TG_CHAT_ID").expect("TG_CHAT_ID must be set");
            debug!("chat_id: {}", chat_id);

            if !tg_message_id.is_empty() {
                debug!(
                    "Try to delete telegram message: chat_id={}, message_id={}",
                    chat_id, tg_message_id
                );
                match bot.delete_message(tg_message_id.clone()).await {
                    Ok(_) => debug!("Telegram message deleted."),
                    Err(e) => debug!("Telegram message delete failed: {}", e),
                }
            }

            match db.del_record_by_id(file_id) {
                Ok(_) => {
                    debug!("DB record deleted: {}", file_id);
                    HttpResponse::Ok().json(serde_json::json!({
                        "message": "Deleted (db+telegram)"
                    }))
                }
                Err(e) => {
                    error!("Delete failed: {}", e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": e.to_string()
                    }))
                }
            }
        }
        Ok(None) => {
            error!("File not found in database: {}", file_id);
            HttpResponse::NotFound().json(serde_json::json!({
                "detail": "File not found in database"
            }))
        }
        Err(e) => {
            error!("Delete failed: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": e.to_string()
            }))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "debug");
    }
    dotenv().ok();
    env_logger::init();
    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "DELETE"])
            .allowed_headers(vec![actix_web::http::header::CONTENT_TYPE])
            .supports_credentials();
        App::new()
            .wrap(cors)
            .service(get_updates)
            .service(upload_file)
            .service(get_files)
            .service(get_file)
            .service(delete_file)
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}

// mod telegram;
// use telegram::api::TelegramBot;
// mod db;
// use db::db::Database;

// use crate::telegram::Bot;
// use dotenv::dotenv;
// use env_logger;
// #[allow(unused_imports)]
// use log::{Level, debug, error, info, warn};
// use std::{env, fs::File};

// #[tokio::main]
// async fn main() {
//     unsafe {
//         std::env::set_var("RUST_LOG", "debug");
//     }
//     dotenv().ok();
//     env_logger::init();

//     // let token = env::var("TG_BOT_TOKEN").unwrap_or_else(|e| {
//     //     error!("TG_BOT_TOKEN must be set in the environment: {}", e);
//     //     std::process::exit(1);
//     // });
//     // let chat_id = env::var("TG_CHAT_ID").unwrap_or_else(|e| {
//     //     error!("TG_CHAT_ID must be set in the environment: {}", e);
//     //     std::process::exit(1);
//     // });
//     // let bot = TelegramBot::new(&token, &chat_id);
//     // let result = bot
//     //     .send_document(File::open("Cargo.toml").unwrap(), "Cargo.toml")
//     //     .await
//     //     .unwrap();
//     // println!("{:?}", result);

//     let db = Database::new("db.db").init_db().unwrap();
// }
