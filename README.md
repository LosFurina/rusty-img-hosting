# Rusty Image Hosting

This is a simple image/file hosting server built with Rust, Actix-Web, and SQLite. It supports file uploads, Telegram integration, and basic file management via RESTful APIs.

## Features
- Upload files via HTTP POST (multipart/form-data)
- Store file metadata in SQLite
- Send uploaded files to a Telegram chat via bot
- List all uploaded files
- Download files by date and UUID
- Delete files (removes from DB and Telegram)
- CORS support for web clients

## Endpoints

### `POST /upload`
Upload a file. The file will be sent to Telegram and stored in the database.

### `GET /files`
List all uploaded files and their metadata.

### `GET /find/{year}/{month}/{day}/{uuid}`
Download a file by its date and UUID.

### `DELETE /files/{file_id}`
Delete a file by its database ID. Also deletes the Telegram message if possible.

### `GET /getUpdates`
Fetch latest updates from the Telegram bot (for debugging).

## Environment Variables
Create a `.env` file in the project root with the following:

```
TG_BOT_TOKEN=your_telegram_bot_token
TG_CHAT_ID=your_telegram_chat_id
```

## Running

1. Install Rust and Cargo.
2. Install dependencies:
   ```
   cargo build
   ```
3. Run the server:
   ```
   cargo run
   ```
4. The server will start at `http://127.0.0.1:8000`.

## Project Structure
- `src/` - Main source code
- `db/` - SQLite database logic
- `telegram/` - Telegram bot API integration
- `public/` - Static web files (optional)

## Notes
- Requires a running Telegram bot and chat.
- The SQLite database file is `db.db` by default.
- Logging is enabled at debug level.

## License
MIT
