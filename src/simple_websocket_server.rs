// =============================================================================
// 簡単なWebSocketサーバー実装（マルチプレイソリティア用）
// =============================================================================
// シンプルで実用的なWebSocketサーバーを実装します。
// プレイヤー間のリアルタイム通信を実現します。
// =============================================================================

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream};
use futures_util::{SinkExt, StreamExt};
use uuid::Uuid;

// =============================================================================
// データ構造定義
// =============================================================================

/// プレイヤー情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub cursor_x: f64,
    pub cursor_y: f64,
    pub color_index: u8,
}

impl Player {
    pub fn new(name: String, color_index: u8) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            cursor_x: 0.0,
            cursor_y: 0.0,
            color_index,
        }
    }
}

/// WebSocketメッセージタイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    PlayerJoin {
        player_id: String,
        player_name: String,
        player_index: u8,
    },
    PlayerLeft {
        player_id: String,
        player_name: String,
    },
    MousePosition {
        player_id: String,
        x: f64,
        y: f64,
        timestamp: u64,
    },
    GameAction {
        player_id: String,
        player_name: String,
        action: String,
        x: Option<f64>,
        y: Option<f64>,
        timestamp: u64,
    },
    Error {
        message: String,
    },
}

// =============================================================================
// 簡単なサーバー実装
// =============================================================================

type Players = Arc<Mutex<HashMap<String, Player>>>;
type Senders = Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>>;

pub struct SimpleWebSocketServer {
    players: Players,
    senders: Senders,
    next_color_index: Arc<Mutex<u8>>,
}

impl SimpleWebSocketServer {
    pub fn new() -> Self {
        Self {
            players: Arc::new(Mutex::new(HashMap::new())),
            senders: Arc::new(Mutex::new(HashMap::new())),
            next_color_index: Arc::new(Mutex::new(1)),
        }
    }

    /// サーバーを開始
    pub async fn start(&self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        println!("🌐 シンプルWebSocketサーバーを{}で開始しました", addr);

        while let Ok((stream, addr)) = listener.accept().await {
            println!("🔗 新しい接続: {}", addr);
            
            let players = Arc::clone(&self.players);
            let senders = Arc::clone(&self.senders);
            let next_color_index = Arc::clone(&self.next_color_index);

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, players, senders, next_color_index).await {
                    println!("❌ 接続処理エラー: {}", e);
                }
            });
        }

        Ok(())
    }

    /// 個別の接続を処理
    async fn handle_connection(
        stream: TcpStream,
        players: Players,
        senders: Senders,
        next_color_index: Arc<Mutex<u8>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ws_stream = accept_async(stream).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let mut player_id: Option<String> = None;

        // 送信タスクを別途起動
        let sender_task = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if ws_sender.send(Message::Text(message)).await.is_err() {
                    break;
                }
            }
        });

        // メッセージ受信ループ
        while let Some(message) = ws_receiver.next().await {
            match message? {
                Message::Text(text) => {
                    println!("📥 受信メッセージ: {}", text);
                    
                    match serde_json::from_str::<WebSocketMessage>(&text) {
                        Ok(msg) => {
                            match msg {
                                WebSocketMessage::PlayerJoin { player_name, player_id: _, player_index: _ } => {
                                    // カラーインデックスを割り当て
                                    let color_index = {
                                        let mut color = next_color_index.lock().unwrap();
                                        let current = *color;
                                        *color = (*color % 5) + 1; // 1-5の循環
                                        current
                                    };
                                    
                                    // 新しいプレイヤーを作成
                                    let player = Player::new(player_name.clone(), color_index);
                                    player_id = Some(player.id.clone());
                                    
                                    // プレイヤーリストに追加
                                    {
                                        let mut players_map = players.lock().unwrap();
                                        players_map.insert(player.id.clone(), player.clone());
                                    }
                                    
                                    // 送信チャンネルに追加
                                    {
                                        let mut senders_map = senders.lock().unwrap();
                                        senders_map.insert(player.id.clone(), tx.clone());
                                    }
                                    
                                    println!("👤 プレイヤー参加: {} ({})", player.name, player.id);
                                    
                                    // 他のプレイヤーに通知
                                    Self::broadcast_to_others(
                                        &WebSocketMessage::PlayerJoin {
                                            player_id: player.id.clone(),
                                            player_name: player.name.clone(),
                                            player_index: player.color_index,
                                        },
                                        &senders,
                                        &player.id
                                    ).await;
                                }
                                
                                WebSocketMessage::MousePosition { player_id: msg_player_id, x, y, timestamp } => {
                                    // プレイヤーのマウス位置を更新
                                    {
                                        let mut players_map = players.lock().unwrap();
                                        if let Some(player) = players_map.get_mut(&msg_player_id) {
                                            player.cursor_x = x;
                                            player.cursor_y = y;
                                        }
                                    }
                                    
                                    // 他のプレイヤーに位置をブロードキャスト
                                    Self::broadcast_to_others(
                                        &WebSocketMessage::MousePosition {
                                            player_id: msg_player_id.clone(),
                                            x,
                                            y,
                                            timestamp,
                                        },
                                        &senders,
                                        &msg_player_id
                                    ).await;
                                }
                                
                                WebSocketMessage::GameAction { player_id: msg_player_id, player_name, action, x, y, timestamp } => {
                                    println!("🎯 ゲームアクション: {} by {}", action, player_name);
                                    
                                    // 他のプレイヤーにアクションをブロードキャスト
                                    Self::broadcast_to_others(
                                        &WebSocketMessage::GameAction {
                                            player_id: msg_player_id.clone(),
                                            player_name,
                                            action,
                                            x,
                                            y,
                                            timestamp,
                                        },
                                        &senders,
                                        &msg_player_id
                                    ).await;
                                }
                                
                                _ => {
                                    println!("⚠️ 未対応メッセージタイプ: {:?}", msg);
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ メッセージパースエラー: {}", e);
                        }
                    }
                }
                Message::Close(_) => {
                    println!("🔌 接続クローズ");
                    break;
                }
                _ => {}
            }
        }

        // クリーンアップ処理
        if let Some(pid) = player_id {
            let player_name = {
                let mut players_map = players.lock().unwrap();
                if let Some(player) = players_map.remove(&pid) {
                    player.name
                } else {
                    "Unknown".to_string()
                }
            };
            
            {
                let mut senders_map = senders.lock().unwrap();
                senders_map.remove(&pid);
            }
            
            println!("👋 プレイヤー退出: {} ({})", player_name, pid);
            
            // 他のプレイヤーに退出を通知
            Self::broadcast_to_others(
                &WebSocketMessage::PlayerLeft {
                    player_id: pid,
                    player_name,
                },
                &senders,
                ""
            ).await;
        }

        // 送信タスクを終了
        sender_task.abort();

        Ok(())
    }

    /// 他のプレイヤーにメッセージをブロードキャスト
    async fn broadcast_to_others(
        message: &WebSocketMessage,
        senders: &Senders,
        exclude_player_id: &str,
    ) {
        let message_text = match serde_json::to_string(message) {
            Ok(text) => text,
            Err(e) => {
                println!("❌ メッセージシリアライゼーションエラー: {}", e);
                return;
            }
        };

        let senders_map = senders.lock().unwrap();
        for (player_id, sender) in senders_map.iter() {
            if player_id != exclude_player_id {
                if let Err(_) = sender.send(message_text.clone()) {
                    println!("⚠️ プレイヤー{}への送信失敗", player_id);
                }
            }
        }
    }
}

// =============================================================================
// サーバー起動用のメイン関数
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 マルチプレイソリティア Simple WebSocketサーバー起動中...");
    
    let server = SimpleWebSocketServer::new();
    server.start("162.43.8.148:8101").await?;
    
    Ok(())
}