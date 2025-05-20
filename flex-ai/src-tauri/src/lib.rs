use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String, // "user" | "assistant" | "system"
    content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatInput {
    messages: Vec<ChatMessage>, // 履歴を受け取る
    model: Option<String>,      // gpt-3.5-turbo など
    temperature: Option<f32>,   // 設定可能
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn stream_chat(input: ChatInput, app_handle: tauri::AppHandle) -> Result<(), String> {
    let client = reqwest::Client::new();
    let mut final_messages = input.messages.clone();
    final_messages.insert(0, ChatMessage {
        role: String::from("system"),
        content: String::from(
            "あなたは優秀プログラマーです。プログラミングのことを聞かれたらコード例を必ず示してください。エラーの内容があれば解決できるコードを提供すること。Markdown形式で返してください。ユーザーに次に質問することを具体的な質問例など示して誘導してください。改行使って読みやすくしてください。",
        )});

    // .envファイルからAPIキーを読み込む
    dotenvy::dotenv().ok();
    let api_key = std::env::var("API_KEY").map_err(|e| format!("API_KEY not set: {}", e))?;
    let req_body = serde_json::json!({
        "model": input.model.unwrap_or("gpt-4.1-mini".into()),
        "messages": final_messages,
        "temperature": input.temperature.unwrap_or(1.0),
        "stream": true
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&req_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    println!("response:{:?}", response);
    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        if let Ok(chunk) = item {
            for line in chunk.split(|b| *b == b'\n') {
                if line.starts_with(b"data: ") {
                    let json = &line[6..];
                    if json == b"[DONE]" {
                        app_handle.emit("chat_token", "[DONE]").unwrap();
                        return Ok(());
                    }

                    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(json) {
                        if let Some(content) = value["choices"][0]["delta"]["content"].as_str() {
                            app_handle.emit("chat_token", content).unwrap();
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, stream_chat])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// #[tokio::main]
// #[test]
// async fn api_test() -> Result<(), Box<dyn std::error::Error>> {
//     let api_key = "***REMOVED***"; // ← あなたのAPIキー

//     let client = reqwest::Client::new();
//     let request_body = ChatRequest {
//         model: "gpt-3.5-turbo".to_string(),
//         messages: vec![ChatMessage {
//             role: "user".to_string(),
//             content: "ストリーミングで応答して".to_string(),
//         }],
//         stream: true,
//     };

//     let mut response = client
//         .post("https://api.openai.com/v1/chat/completions")
//         .bearer_auth(api_key)
//         .json(&request_body)
//         .send()
//         .await?
//         .bytes_stream();

//     println!("Streaming response:");

//     while let Some(item) = response.next().await {
//         let chunk = item?;
//         for line in chunk.split(|b| *b == b'\n') {
//             if line.starts_with(b"data: ") {
//                 let json_line = &line[6..]; // "data: " を除去
//                 if json_line == b"[DONE]" {
//                     println!("\n[Done]");
//                     return Ok(());
//                 }
//                 if let Ok(parsed) = serde_json::from_slice::<StreamChunk>(json_line) {
//                     for choice in parsed.choices {
//                         if let Some(content) = choice.delta.content {
//                             print!("{}", content);
//                         }
//                     }
//                 }
//             }
//         }
//     }

//     Ok(())
// }
