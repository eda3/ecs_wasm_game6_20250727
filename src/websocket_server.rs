// =============================================================================
// WebSocketサーバー実装
// =============================================================================
// このファイルでは、マルチプレイソリティアゲーム用のWebSocketサーバーを実装します。
// tokio-tungsteniteを使用してリアルタイム通信を実現し、
// 複数のプレイヤー間でゲーム状態やマウスカーソル位置を同期します。
//
// 主要な機能：
// - プレイヤーの接続・切断管理
// - マウスカーソル位置のリアルタイム同期
// - ゲームアクションのブロードキャスト
// - 部屋（Room）システムによるマルチプレイ管理
// =============================================================================

use std::collections::HashMap;
use std::net::SocketAddr;
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
    pub room_id: Option<String>,
    pub cursor_x: f64,
    pub cursor_y: f64,
    pub is_connected: bool,
    pub color_index: u8, // カーソル色用のインデックス
}

impl Player {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            room_id: None,
            cursor_x: 0.0,
            cursor_y: 0.0,
            is_connected: true,
            color_index: 1,
        }
    }
}

/// ゲームルーム情報
#[derive(Debug, Clone)]
pub struct GameRoom {
    pub id: String,
    pub name: String,
    pub players: Vec<String>, // プレイヤーIDのリスト
    pub max_players: u8,
    pub game_state: GameState,
    pub created_at: std::time::SystemTime,
}

impl GameRoom {
    pub fn new(name: String, max_players: u8) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            players: Vec::new(),
            max_players,
            game_state: GameState::Waiting,
            created_at: std::time::SystemTime::now(),
        }
    }

    pub fn add_player(&mut self, player_id: String) -> bool {
        if self.players.len() < self.max_players as usize && !self.players.contains(&player_id) {
            self.players.push(player_id);
            true
        } else {
            false
        }
    }

    pub fn remove_player(&mut self, player_id: &str) -> bool {
        if let Some(pos) = self.players.iter().position(|x| x == player_id) {
            self.players.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn is_full(&self) -> bool {
        self.players.len() >= self.max_players as usize
    }
}

/// ゲーム状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameState {
    Waiting,    // プレイヤー待機中
    Playing,    // ゲーム進行中
    Finished,   // ゲーム終了
}

/// WebSocketメッセージタイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    // 接続関連
    PlayerJoin {
        player_id: String,
        player_name: String,
        player_index: u8,
    },
    PlayerLeft {
        player_id: String,
        player_name: String,
    },
    
    // マウスカーソル関連
    MousePosition {
        player_id: String,
        x: f64,
        y: f64,
        timestamp: u64,
    },
    
    // ゲームアクション関連
    GameAction {
        player_id: String,
        player_name: String,
        action: String,
        x: Option<f64>,
        y: Option<f64>,
        timestamp: u64,
    },
    
    // ルーム関連
    JoinRoom {
        room_id: String,
        player_id: String,
    },
    LeaveRoom {
        room_id: String,
        player_id: String,
    },
    RoomList {
        rooms: Vec<RoomInfo>,
    },
    
    // エラー
    Error {
        message: String,
    },
}

/// ルーム情報（クライアント送信用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub id: String,
    pub name: String,
    pub player_count: u8,
    pub max_players: u8,
    pub game_state: GameState,
}

// =============================================================================
// サーバーメイン構造体
// =============================================================================

type Players = Arc<Mutex<HashMap<String, Player>>>;
type Rooms = Arc<Mutex<HashMap<String, GameRoom>>>;
type Connections = Arc<Mutex<HashMap<String, WebSocketStream<TcpStream>>>>;

pub struct SolitaireServer {
    players: Players,
    rooms: Rooms,
    connections: Connections,
    next_color_index: Arc<Mutex<u8>>,
}

impl SolitaireServer {
    pub fn new() -> Self {
        Self {
            players: Arc::new(Mutex::new(HashMap::new())),
            rooms: Arc::new(Mutex::new(HashMap::new())),
            connections: Arc::new(Mutex::new(HashMap::new())),
            next_color_index: Arc::new(Mutex::new(1)),
        }
    }

    /// サーバーを開始
    pub async fn start(&self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        println!("🌐 WebSocketサーバーを{}で開始しました", addr);

        // デフォルトルームを作成
        self.create_default_room().await;

        while let Ok((stream, addr)) = listener.accept().await {
            println!("🔗 新しい接続: {}", addr);
            
            let players = Arc::clone(&self.players);
            let rooms = Arc::clone(&self.rooms);
            let connections = Arc::clone(&self.connections);
            let next_color_index = Arc::clone(&self.next_color_index);

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, addr, players, rooms, connections, next_color_index).await {
                    println!("❌ 接続処理エラー: {}", e);
                }
            });
        }

        Ok(())
    }

    /// デフォルトルームを作成
    async fn create_default_room(&self) {
        let mut rooms = self.rooms.lock().unwrap();
        let default_room = GameRoom::new("メインルーム".to_string(), 4);
        rooms.insert(default_room.id.clone(), default_room);
        println!("🏠 デフォルトルームを作成しました");
    }

    /// 個別の接続を処理
    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        players: Players,
        rooms: Rooms,
        connections: Connections,
        next_color_index: Arc<Mutex<u8>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ws_stream = accept_async(stream).await?;
        let (ws_sender, mut ws_receiver) = ws_stream.split();
        
        // 送信用のsenderをArcで包む
        let ws_sender = Arc::new(Mutex::new(ws_sender));

        let mut player_id: Option<String> = None;

        while let Some(message) = ws_receiver.next().await {
            match message? {
                Message::Text(text) => {
                    println!("📥 受信メッセージ: {}", text);
                    
                    match serde_json::from_str::<WebSocketMessage>(&text) {
                        Ok(msg) => {
                            match msg {
                                WebSocketMessage::PlayerJoin { player_name, .. } => {
                                    // 新しいプレイヤーを作成
                                    let mut player = Player::new(player_name.clone());
                                    
                                    // カラーインデックスを割り当て
                                    {
                                        let mut color_index = next_color_index.lock().unwrap();
                                        player.color_index = *color_index;
                                        *color_index = (*color_index % 5) + 1; // 1-5の循環
                                    }
                                    
                                    player_id = Some(player.id.clone());
                                    
                                    // プレイヤーリストに追加
                                    {
                                        let mut players_map = players.lock().unwrap();
                                        players_map.insert(player.id.clone(), player.clone());
                                    }
                                    
                                    println!("👤 プレイヤー参加: {} ({})", player.name, player.id);
                                    
                                    // 他のプレイヤーに通知
                                    Self::broadcast_to_all(
                                        &WebSocketMessage::PlayerJoin {
                                            player_id: player.id.clone(),
                                            player_name: player.name.clone(),
                                            player_index: player.color_index,
                                        },
                                        &connections,
                                        Some(&player.id)
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
                                    Self::broadcast_to_all(
                                        &WebSocketMessage::MousePosition {
                                            player_id: msg_player_id.clone(),
                                            x,
                                            y,
                                            timestamp,
                                        },
                                        &connections,
                                        Some(&msg_player_id)
                                    ).await;
                                }
                                
                                WebSocketMessage::GameAction { player_id: msg_player_id, player_name, action, x, y, timestamp } => {
                                    println!("🎯 ゲームアクション: {} by {}", action, player_name);
                                    
                                    // 他のプレイヤーにアクションをブロードキャスト
                                    Self::broadcast_to_all(
                                        &WebSocketMessage::GameAction {
                                            player_id: msg_player_id.clone(),
                                            player_name,
                                            action,
                                            x,
                                            y,
                                            timestamp,
                                        },
                                        &connections,
                                        Some(&msg_player_id)
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
                    println!("🔌 接続クローズ: {}", addr);
                    break;
                }
                _ => {}
            }
        }

        // プレイヤーが切断した場合のクリーンアップ
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
                let mut connections_map = connections.lock().unwrap();
                connections_map.remove(&pid);
            }
            
            println!("👋 プレイヤー退出: {} ({})", player_name, pid);
            
            // 他のプレイヤーに退出を通知
            Self::broadcast_to_all(
                &WebSocketMessage::PlayerLeft {
                    player_id: pid,
                    player_name,
                },
                &connections,
                None
            ).await;
        }

        Ok(())
    }

    /// 全プレイヤーにメッセージをブロードキャスト
    async fn broadcast_to_all(
        message: &WebSocketMessage,
        connections: &Connections,
        exclude_player: Option<&str>,
    ) {
        let message_text = match serde_json::to_string(message) {
            Ok(text) => text,
            Err(e) => {
                println!("❌ メッセージシリアライゼーションエラー: {}", e);
                return;
            }
        };

        let connections_map = connections.lock().unwrap();
        for (player_id, _connection) in connections_map.iter() {
            if let Some(exclude) = exclude_player {
                if player_id == exclude {
                    continue;
                }
            }
            
            // 実際の送信は実装の都合上省略（tokio-tungsteniteの使用方法による）
            println!("📤 ブロードキャスト -> {}: {}", player_id, message_text);
        }
    }
}

// =============================================================================
// サーバー起動用のメイン関数
// =============================================================================

pub async fn run_websocket_server() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 マルチプレイソリティア WebSocketサーバー起動中...");
    
    let server = SolitaireServer::new();
    server.start("162.43.8.148:8101").await?;
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_websocket_server().await
}