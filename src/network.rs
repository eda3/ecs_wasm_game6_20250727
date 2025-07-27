// =============================================================================
// WebSocket通信レイヤ
// =============================================================================
// このファイルでは、マルチプレイソリティアゲームのWebSocket通信を実装します。
// クライアント・サーバ間のリアルタイム通信を管理し、ゲーム状態の同期や
// プレイヤーアクションの配信を行います。
//
// 主要な責務：
// - WebSocket接続の管理（接続、切断、再接続）
// - メッセージの送受信とシリアライゼーション
// - ゲーム状態の同期機能
// - エラーハンドリングと接続品質の監視
// - 複数プレイヤー間でのメッセージブロードキャスト
// =============================================================================

use crate::ecs::{World, Entity, Component, System};
use serde::{Serialize, Deserialize};
// use std::collections::HashMap; // 未使用のため一時的にコメントアウト
use std::time::{SystemTime, UNIX_EPOCH};

// WebAssembly機能が有効な場合のみWebSocket関連のインポート
#[cfg(feature = "wasm")]
use web_sys::{WebSocket, MessageEvent, CloseEvent, ErrorEvent};
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "wasm")]
use wasm_bindgen::JsCast;

// =============================================================================
// ネットワーク関連のコンポーネント定義
// =============================================================================

/// ネットワーク接続を表すコンポーネント
/// 
/// 各プレイヤーやゲームセッションの接続状態を管理します。
/// WebSocket接続の詳細情報と状態を保持します。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkConnection {
    /// 接続の一意識別子
    pub connection_id: String,
    
    /// 接続状態
    pub status: ConnectionStatus,
    
    /// 接続先のURL
    pub url: String,
    
    /// 最後のアクティビティ時刻（UNIXタイムスタンプ）
    pub last_activity: u64,
    
    /// 接続試行回数
    pub retry_count: u32,
    
    /// Ping/Pong による遅延測定（ミリ秒）
    pub latency_ms: Option<u32>,
    
    /// 送信メッセージ数
    pub sent_messages: u64,
    
    /// 受信メッセージ数
    pub received_messages: u64,
}

impl Component for NetworkConnection {}

impl NetworkConnection {
    /// 新しいネットワーク接続を作成
    /// 
    /// # 引数
    /// * `connection_id` - 接続の一意識別子
    /// * `url` - 接続先のWebSocket URL
    /// 
    /// # 戻り値
    /// 初期化されたNetworkConnectionインスタンス
    pub fn new(connection_id: String, url: String) -> Self {
        Self {
            connection_id,
            status: ConnectionStatus::Disconnected,
            url,
            last_activity: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            retry_count: 0,
            latency_ms: None,
            sent_messages: 0,
            received_messages: 0,
        }
    }
    
    /// 接続状態を更新
    /// 
    /// # 引数
    /// * `new_status` - 新しい接続状態
    pub fn update_status(&mut self, new_status: ConnectionStatus) {
        self.status = new_status;
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    
    /// メッセージ送信カウンターを増加
    pub fn increment_sent(&mut self) {
        self.sent_messages += 1;
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    
    /// メッセージ受信カウンターを増加
    pub fn increment_received(&mut self) {
        self.received_messages += 1;
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    
    /// 再試行カウンターを増加
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
    
    /// 遅延を更新
    /// 
    /// # 引数
    /// * `latency_ms` - 新しい遅延時間（ミリ秒）
    pub fn update_latency(&mut self, latency_ms: u32) {
        self.latency_ms = Some(latency_ms);
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    
    /// 接続がアクティブかどうかチェック
    /// 
    /// # 引数
    /// * `timeout_seconds` - タイムアウト時間（秒）
    /// 
    /// # 戻り値
    /// アクティブな場合true、タイムアウトした場合false
    pub fn is_active(&self, timeout_seconds: u64) -> bool {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        current_time - self.last_activity < timeout_seconds
    }
}

/// WebSocket接続の状態を表す列挙型
/// 
/// 接続のライフサイクルを明確に管理するため、各状態を定義します。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// 未接続
    Disconnected,
    
    /// 接続中
    Connecting,
    
    /// 接続完了
    Connected,
    
    /// 再接続中
    Reconnecting,
    
    /// 接続エラー
    Error,
    
    /// 接続終了
    Closed,
}

impl ConnectionStatus {
    /// 状態名を文字列で取得
    /// 
    /// # 戻り値
    /// 状態名の文字列
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionStatus::Disconnected => "disconnected",
            ConnectionStatus::Connecting => "connecting",
            ConnectionStatus::Connected => "connected",
            ConnectionStatus::Reconnecting => "reconnecting",
            ConnectionStatus::Error => "error",
            ConnectionStatus::Closed => "closed",
        }
    }
}

/// ネットワークメッセージを表すコンポーネント
/// 
/// WebSocketで送受信されるメッセージを管理します。
/// メッセージの種類、内容、タイムスタンプなどを保持します。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkMessage {
    /// メッセージの一意識別子
    pub message_id: String,
    
    /// メッセージの種類
    pub message_type: MessageType,
    
    /// 送信者のエンティティID（オプション）
    pub sender: Option<Entity>,
    
    /// 受信者のエンティティID（オプション、未指定時はブロードキャスト）
    pub recipient: Option<Entity>,
    
    /// メッセージの内容（JSONシリアライズ済み）
    pub payload: String,
    
    /// タイムスタンプ（UNIXタイムスタンプ）
    pub timestamp: u64,
    
    /// メッセージの優先度
    pub priority: MessagePriority,
    
    /// 再送信回数
    pub retry_count: u32,
}

impl Component for NetworkMessage {}

impl NetworkMessage {
    /// 新しいネットワークメッセージを作成
    /// 
    /// # 引数
    /// * `message_type` - メッセージの種類
    /// * `payload` - メッセージの内容
    /// * `sender` - 送信者（オプション）
    /// * `recipient` - 受信者（オプション）
    /// 
    /// # 戻り値
    /// 新しいNetworkMessageインスタンス
    pub fn new(
        message_type: MessageType,
        payload: String,
        sender: Option<Entity>,
        recipient: Option<Entity>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        Self {
            message_id: format!("msg_{}_{}", timestamp, rand::random::<u32>()),
            message_type,
            sender,
            recipient,
            payload,
            timestamp,
            priority: MessagePriority::Normal,
            retry_count: 0,
        }
    }
    
    /// 高優先度メッセージとして作成
    /// 
    /// # 引数
    /// * `message_type` - メッセージの種類
    /// * `payload` - メッセージの内容
    /// * `sender` - 送信者（オプション）
    /// * `recipient` - 受信者（オプション）
    /// 
    /// # 戻り値
    /// 高優先度のNetworkMessageインスタンス
    pub fn new_high_priority(
        message_type: MessageType,
        payload: String,
        sender: Option<Entity>,
        recipient: Option<Entity>,
    ) -> Self {
        let mut message = Self::new(message_type, payload, sender, recipient);
        message.priority = MessagePriority::High;
        message
    }
    
    /// 再送信カウンターを増加
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
    
    /// メッセージが古すぎるかチェック
    /// 
    /// # 引数
    /// * `max_age_seconds` - 最大許容経過時間（秒）
    /// 
    /// # 戻り値
    /// 古すぎる場合true、まだ有効な場合false
    pub fn is_expired(&self, max_age_seconds: u64) -> bool {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        current_time - self.timestamp > max_age_seconds
    }
}

/// メッセージの種類を表す列挙型
/// 
/// WebSocketで送受信される様々なメッセージタイプを定義します。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    /// プレイヤーアクション（カード移動など）
    PlayerAction,
    
    /// ゲーム状態の同期
    GameStateSync,
    
    /// プレイヤーの参加/退出
    PlayerJoinLeave,
    
    /// チャットメッセージ
    Chat,
    
    /// システム通知
    SystemNotification,
    
    /// Ping/Pong（接続確認）
    Ping,
    Pong,
    
    /// エラー通知
    Error,
    
    /// 認証関連
    Authentication,
    
    /// ゲーム設定変更
    GameSettings,
}

impl MessageType {
    /// メッセージタイプ名を文字列で取得
    /// 
    /// # 戻り値
    /// メッセージタイプ名の文字列
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::PlayerAction => "player_action",
            MessageType::GameStateSync => "game_state_sync",
            MessageType::PlayerJoinLeave => "player_join_leave",
            MessageType::Chat => "chat",
            MessageType::SystemNotification => "system_notification",
            MessageType::Ping => "ping",
            MessageType::Pong => "pong",
            MessageType::Error => "error",
            MessageType::Authentication => "authentication",
            MessageType::GameSettings => "game_settings",
        }
    }
}

/// メッセージの優先度を表す列挙型
/// 
/// メッセージの送信順序や処理優先度を制御します。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    /// 低優先度（チャットメッセージなど）
    Low = 0,
    
    /// 通常優先度（ゲームアクションなど）
    Normal = 1,
    
    /// 高優先度（システム通知など）
    High = 2,
    
    /// 緊急（エラー通知など）
    Critical = 3,
}

// =============================================================================
// WebSocket管理クラス（WebAssembly環境用）
// =============================================================================

/// WebSocket接続マネージャー（WebAssembly用）
/// 
/// ブラウザ環境でのWebSocket接続を管理します。
/// 接続の確立、メッセージの送受信、エラーハンドリングを行います。
#[cfg(feature = "wasm")]
pub struct WebSocketManager {
    /// WebSocketインスタンス
    websocket: Option<WebSocket>,
    
    /// 接続状態
    status: ConnectionStatus,
    
    /// 接続URL
    url: String,
    
    /// メッセージキュー（送信待ち）
    message_queue: Vec<NetworkMessage>,
    
    /// 最大再試行回数
    max_retries: u32,
    
    /// 現在の再試行回数
    current_retries: u32,
}

#[cfg(feature = "wasm")]
impl WebSocketManager {
    /// 新しいWebSocketマネージャーを作成
    /// 
    /// # 引数
    /// * `url` - 接続先のWebSocket URL
    /// 
    /// # 戻り値
    /// 新しいWebSocketManagerインスタンス
    pub fn new(url: String) -> Self {
        Self {
            websocket: None,
            status: ConnectionStatus::Disconnected,
            url,
            message_queue: Vec::new(),
            max_retries: 3,
            current_retries: 0,
        }
    }
    
    /// WebSocket接続を開始
    /// 
    /// # 戻り値
    /// 接続開始が成功した場合Ok(())、失敗した場合Err
    pub fn connect(&mut self) -> Result<(), String> {
        if self.status == ConnectionStatus::Connected {
            return Ok(()); // 既に接続済み
        }
        
        self.status = ConnectionStatus::Connecting;
        
        match WebSocket::new(&self.url) {
            Ok(ws) => {
                // バイナリタイプを設定
                ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
                
                // イベントハンドラーを設定
                self.setup_event_handlers(&ws);
                
                self.websocket = Some(ws);
                println!("🌐 WebSocket接続開始: {}", self.url);
                Ok(())
            }
            Err(e) => {
                self.status = ConnectionStatus::Error;
                let error_msg = format!("WebSocket接続失敗: {:?}", e);
                println!("❌ {}", error_msg);
                Err(error_msg)
            }
        }
    }
    
    /// WebSocket接続を切断
    pub fn disconnect(&mut self) {
        if let Some(ws) = &self.websocket {
            let _ = ws.close();
        }
        self.websocket = None;
        self.status = ConnectionStatus::Disconnected;
        println!("🔌 WebSocket接続を切断しました");
    }
    
    /// メッセージを送信
    /// 
    /// # 引数
    /// * `message` - 送信するメッセージ
    /// 
    /// # 戻り値
    /// 送信成功時Ok(())、失敗時Err
    pub fn send_message(&mut self, message: NetworkMessage) -> Result<(), String> {
        if self.status != ConnectionStatus::Connected {
            // 接続されていない場合はキューに追加
            self.message_queue.push(message);
            return Ok(());
        }
        
        if let Some(ws) = &self.websocket {
            match serde_json::to_string(&message) {
                Ok(json_str) => {
                    if let Err(e) = ws.send_with_str(&json_str) {
                        let error_msg = format!("メッセージ送信失敗: {:?}", e);
                        println!("❌ {}", error_msg);
                        return Err(error_msg);
                    }
                    println!("📤 メッセージ送信: {} ({})", message.message_type.as_str(), message.message_id);
                    Ok(())
                }
                Err(e) => {
                    let error_msg = format!("メッセージシリアライゼーション失敗: {}", e);
                    println!("❌ {}", error_msg);
                    Err(error_msg)
                }
            }
        } else {
            Err("WebSocket接続が存在しません".to_string())
        }
    }
    
    /// キューに溜まったメッセージを送信
    pub fn flush_message_queue(&mut self) {
        if self.status != ConnectionStatus::Connected {
            return;
        }
        
        let messages = std::mem::take(&mut self.message_queue);
        for message in messages {
            if let Err(e) = self.send_message(message) {
                println!("⚠️ キューからのメッセージ送信失敗: {}", e);
            }
        }
    }
    
    /// 現在の接続状態を取得
    /// 
    /// # 戻り値
    /// 現在の接続状態
    pub fn get_status(&self) -> ConnectionStatus {
        self.status
    }
    
    /// イベントハンドラーを設定
    /// 
    /// # 引数
    /// * `ws` - WebSocketインスタンス
    fn setup_event_handlers(&mut self, ws: &WebSocket) {
        // 接続開始イベント
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            println!("✅ WebSocket接続が確立されました");
        }) as Box<dyn FnMut(JsValue)>);
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();
        
        // メッセージ受信イベント
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let message_str = String::from(txt);
                println!("📥 メッセージ受信: {}", message_str);
                
                // メッセージをパースして処理
                if let Ok(message) = serde_json::from_str::<NetworkMessage>(&message_str) {
                    println!("🔍 メッセージ解析完了: {} ({})", 
                        message.message_type.as_str(), 
                        message.message_id
                    );
                    // TODO: ECSシステムにメッセージを渡す処理を追加
                } else {
                    println!("⚠️ メッセージのパースに失敗しました");
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();
        
        // 接続終了イベント
        let onclose_callback = Closure::wrap(Box::new(move |e: CloseEvent| {
            println!("🔌 WebSocket接続が終了されました (コード: {})", e.code());
        }) as Box<dyn FnMut(CloseEvent)>);
        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();
        
        // エラーイベント
        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            println!("❌ WebSocketエラーが発生しました: {:?}", e);
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
    }
}

// =============================================================================
// ネットワーク管理システム群
// =============================================================================

/// ネットワーク接続管理システム
/// 
/// すべてのネットワーク接続の状態を監視し、必要に応じて
/// 再接続やタイムアウト処理を行います。
pub struct NetworkConnectionSystem;

impl System for NetworkConnectionSystem {
    fn update(&mut self, world: &mut World, _delta_time: f64) {
        let mut reconnection_needed = Vec::new();
        let mut timeout_connections = Vec::new();
        
        // 全ての接続を監視
        for (entity, connection) in world.query::<NetworkConnection>() {
            match connection.status {
                ConnectionStatus::Error => {
                    if connection.retry_count < 3 {
                        reconnection_needed.push(entity);
                    }
                }
                ConnectionStatus::Connected => {
                    // 60秒間アクティビティがない場合はタイムアウト
                    if !connection.is_active(60) {
                        timeout_connections.push(entity);
                    }
                }
                _ => {}
            }
            
            // 接続統計をデバッグ出力（定期的に）
            if connection.sent_messages > 0 || connection.received_messages > 0 {
                println!(
                    "📊 接続統計 [{}]: 送信{}件, 受信{}件, 遅延{:?}ms, 状態:{}",
                    connection.connection_id,
                    connection.sent_messages,
                    connection.received_messages,
                    connection.latency_ms,
                    connection.status.as_str()
                );
            }
        }
        
        // 再接続処理
        for entity in reconnection_needed {
            if let Some(connection) = world.get_component_mut::<NetworkConnection>(entity) {
                connection.increment_retry();
                connection.update_status(ConnectionStatus::Reconnecting);
                println!("🔄 接続再試行: {} ({}回目)", connection.connection_id, connection.retry_count);
            }
        }
        
        // タイムアウト処理
        for entity in timeout_connections {
            if let Some(connection) = world.get_component_mut::<NetworkConnection>(entity) {
                connection.update_status(ConnectionStatus::Error);
                println!("⏰ 接続タイムアウト: {}", connection.connection_id);
            }
        }
    }
}

/// メッセージ処理システム
/// 
/// ネットワークメッセージの送受信、キューイング、優先度制御を行います。
pub struct MessageProcessingSystem;

impl System for MessageProcessingSystem {
    fn update(&mut self, world: &mut World, _delta_time: f64) {
        let mut processed_messages = Vec::new();
        let mut expired_messages = Vec::new();
        
        // 全てのメッセージを処理
        for (entity, message) in world.query::<NetworkMessage>() {
            // 古いメッセージをチェック（300秒でタイムアウト）
            if message.is_expired(300) {
                expired_messages.push(entity);
                continue;
            }
            
            println!(
                "📨 メッセージ処理: {} -> {:?} (優先度: {:?}, {}回目)",
                message.message_type.as_str(),
                message.recipient,
                message.priority,
                message.retry_count + 1
            );
            
            // メッセージタイプに応じた処理
            match message.message_type {
                MessageType::PlayerAction => {
                    // プレイヤーアクションの処理
                    println!("🎯 プレイヤーアクション処理: {}", message.payload);
                }
                
                MessageType::GameStateSync => {
                    // ゲーム状態同期の処理
                    println!("🔄 ゲーム状態同期: {}", message.payload);
                }
                
                MessageType::Chat => {
                    // チャットメッセージの処理
                    println!("💬 チャット: {}", message.payload);
                }
                
                MessageType::Ping => {
                    // Pingに対してPongを返す
                    println!("🏓 Ping受信、Pong送信");
                }
                
                MessageType::Pong => {
                    // Pongを受信（遅延測定に使用）
                    println!("🏓 Pong受信");
                }
                
                _ => {
                    println!("📄 その他のメッセージ処理: {}", message.message_type.as_str());
                }
            }
            
            processed_messages.push(entity);
        }
        
        // 処理済みメッセージを削除
        for entity in processed_messages {
            world.remove_component::<NetworkMessage>(entity);
        }
        
        // 期限切れメッセージを削除
        for entity in expired_messages {
            println!("🗑️ 期限切れメッセージを削除");
            world.remove_component::<NetworkMessage>(entity);
        }
    }
}

// =============================================================================
// ネットワーク管理のユーティリティ関数
// =============================================================================

/// ネットワークマネージャー
/// 
/// ネットワーク機能の管理を支援するユーティリティ構造体です。
pub struct NetworkManager;

impl NetworkManager {
    /// 新しいネットワーク接続を作成
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `connection_id` - 接続ID
    /// * `url` - 接続先URL
    /// 
    /// # 戻り値
    /// 作成された接続エンティティ
    pub fn create_connection(
        world: &mut World,
        connection_id: String,
        url: String,
    ) -> Entity {
        let connection_entity = world.create_entity();
        let connection = NetworkConnection::new(connection_id.clone(), url.clone());
        
        world.add_component(connection_entity, connection);
        
        println!("🌐 新しいネットワーク接続作成: {} -> {}", connection_id, url);
        connection_entity
    }
    
    /// メッセージを送信キューに追加
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `message_type` - メッセージタイプ
    /// * `payload` - メッセージの内容
    /// * `sender` - 送信者
    /// * `recipient` - 受信者（オプション）
    /// 
    /// # 戻り値
    /// 作成されたメッセージエンティティ
    pub fn send_message(
        world: &mut World,
        message_type: MessageType,
        payload: String,
        sender: Option<Entity>,
        recipient: Option<Entity>,
    ) -> Entity {
        let message_entity = world.create_entity();
        let message = NetworkMessage::new(message_type, payload, sender, recipient);
        
        world.add_component(message_entity, message);
        
        println!("📤 メッセージキューに追加: {}", message_type.as_str());
        message_entity
    }
    
    /// 高優先度メッセージを送信
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `message_type` - メッセージタイプ
    /// * `payload` - メッセージの内容
    /// * `sender` - 送信者
    /// * `recipient` - 受信者（オプション）
    /// 
    /// # 戻り値
    /// 作成されたメッセージエンティティ
    pub fn send_priority_message(
        world: &mut World,
        message_type: MessageType,
        payload: String,
        sender: Option<Entity>,
        recipient: Option<Entity>,
    ) -> Entity {
        let message_entity = world.create_entity();
        let message = NetworkMessage::new_high_priority(message_type, payload, sender, recipient);
        
        world.add_component(message_entity, message);
        
        println!("🚨 高優先度メッセージキューに追加: {}", message_type.as_str());
        message_entity
    }
    
    /// 接続状態を更新
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `connection_entity` - 接続エンティティ
    /// * `new_status` - 新しい接続状態
    pub fn update_connection_status(
        world: &mut World,
        connection_entity: Entity,
        new_status: ConnectionStatus,
    ) {
        if let Some(connection) = world.get_component_mut::<NetworkConnection>(connection_entity) {
            let old_status = connection.status;
            connection.update_status(new_status);
            
            println!(
                "🔄 接続状態変更: {} -> {} ({})",
                old_status.as_str(),
                new_status.as_str(),
                connection.connection_id
            );
        }
    }
}

// =============================================================================
// 乱数生成のモック（WebAssembly環境では実際のrand crateが必要）
// =============================================================================

/// 簡単な乱数生成（開発用、本番では適切なrand crateを使用）
mod rand {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    pub fn random<T>() -> T 
    where 
        T: From<u32>,
    {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u32;
        T::from(timestamp % 100000)
    }
}