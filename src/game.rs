// =============================================================================
// ゲーム状態管理システム
// =============================================================================
// このファイルでは、マルチプレイソリティアゲームの状態管理を実装します。
// ECSアーキテクチャを活用して、ゲームの進行状態、プレイヤーのターン、
// カードの配置、スコア計算などの複雑な状態を効率的に管理します。
//
// 主要な責務：
// - ゲームフェーズの管理（待機、進行中、終了など）
// - プレイヤーターンの制御
// - ゲームルールの適用と検証
// - 勝利条件の判定
// - ゲーム状態の永続化とシリアライゼーション
// =============================================================================

use crate::ecs::{World, Entity, Component, System};
use serde::{Serialize, Deserialize};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

// =============================================================================
// ゲーム状態関連のコンポーネント定義
// =============================================================================

/// ゲーム全体の状態を表すコンポーネント
/// 
/// ゲームセッション全体の進行状況や設定を管理します。
/// 通常、1つのゲームセッションにつき1つのGameStateエンティティが存在します。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameState {
    /// ゲームセッションの一意識別子
    pub session_id: String,
    
    /// 現在のゲームフェーズ
    pub phase: GamePhase,
    
    /// ゲーム開始時刻（UNIXタイムスタンプ）
    pub start_time: u64,
    
    /// 最大プレイヤー数
    pub max_players: u32,
    
    /// 現在参加中のプレイヤー数
    pub current_players: u32,
    
    /// ゲーム設定
    pub settings: GameSettings,
}

impl Component for GameState {}

impl GameState {
    /// 新しいゲーム状態を作成
    /// 
    /// # 引数
    /// * `session_id` - ゲームセッションID
    /// * `max_players` - 最大プレイヤー数
    /// 
    /// # 戻り値
    /// 初期化されたGameStateインスタンス
    pub fn new(session_id: String, max_players: u32) -> Self {
        Self {
            session_id,
            phase: GamePhase::WaitingForPlayers,
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            max_players,
            current_players: 0,
            settings: GameSettings::default(),
        }
    }
    
    /// ゲームを開始できる状態かチェック
    /// 
    /// # 戻り値
    /// 開始可能な場合true、不可能な場合false
    pub fn can_start(&self) -> bool {
        self.phase == GamePhase::WaitingForPlayers 
            && self.current_players >= 2
            && self.current_players <= self.max_players
    }
    
    /// ゲームフェーズを変更
    /// 
    /// # 引数
    /// * `new_phase` - 新しいフェーズ
    pub fn change_phase(&mut self, new_phase: GamePhase) {
        self.phase = new_phase;
    }
    
    /// プレイヤーを追加
    /// 
    /// # 戻り値
    /// 追加成功時true、失敗時false（既に満員の場合など）
    pub fn add_player(&mut self) -> bool {
        if self.current_players < self.max_players {
            self.current_players += 1;
            true
        } else {
            false
        }
    }
    
    /// プレイヤーを削除
    /// 
    /// # 戻り値
    /// 削除成功時true、失敗時false（既に0人の場合など）
    pub fn remove_player(&mut self) -> bool {
        if self.current_players > 0 {
            self.current_players -= 1;
            true
        } else {
            false
        }
    }
}

/// ゲームの進行フェーズを表す列挙型
/// 
/// ゲームの進行状況を明確に管理するため、各フェーズを定義します。
/// 状態遷移は決められた順序でのみ行われます。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GamePhase {
    /// プレイヤーの参加を待機中
    WaitingForPlayers,
    
    /// ゲーム開始準備中（カードの配布など）
    Starting,
    
    /// ゲーム進行中
    Playing,
    
    /// ゲーム一時停止中
    Paused,
    
    /// ゲーム終了（勝者決定）
    Finished,
    
    /// ゲーム中断（エラーや異常終了）
    Aborted,
}

impl GamePhase {
    /// フェーズ名を文字列で取得
    /// 
    /// # 戻り値
    /// フェーズ名の文字列
    pub fn as_str(&self) -> &'static str {
        match self {
            GamePhase::WaitingForPlayers => "waiting_for_players",
            GamePhase::Starting => "starting",
            GamePhase::Playing => "playing",
            GamePhase::Paused => "paused",
            GamePhase::Finished => "finished",
            GamePhase::Aborted => "aborted",
        }
    }
    
    /// 指定されたフェーズに遷移可能かチェック
    /// 
    /// # 引数
    /// * `target` - 遷移先のフェーズ
    /// 
    /// # 戻り値
    /// 遷移可能な場合true、不可能な場合false
    pub fn can_transition_to(&self, target: GamePhase) -> bool {
        use GamePhase::*;
        
        match (self, target) {
            // 待機中から開始準備へ
            (WaitingForPlayers, Starting) => true,
            // 開始準備から進行中へ
            (Starting, Playing) => true,
            // 進行中から一時停止へ
            (Playing, Paused) => true,
            // 一時停止から進行中へ
            (Paused, Playing) => true,
            // 進行中から終了へ
            (Playing, Finished) => true,
            // 任意の状態から中断へ
            (_, Aborted) => true,
            // 終了や中断からは新しいゲームでのみ遷移可能
            (Finished | Aborted, WaitingForPlayers) => true,
            // その他の遷移は不可
            _ => false,
        }
    }
}

/// ゲーム設定を格納する構造体
/// 
/// ゲームの各種設定やルールのカスタマイズを管理します。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameSettings {
    /// 制限時間（秒）。0の場合は制限なし
    pub time_limit: u32,
    
    /// ターン制限時間（秒）
    pub turn_time_limit: u32,
    
    /// デバッグモードの有効/無効
    pub debug_mode: bool,
    
    /// 自動保存の有効/無効
    pub auto_save: bool,
    
    /// 観戦者の許可/禁止
    pub allow_spectators: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            time_limit: 0,          // 制限なし
            turn_time_limit: 30,    // 30秒
            debug_mode: false,
            auto_save: true,
            allow_spectators: true,
        }
    }
}

/// プレイヤーのターン情報を管理するコンポーネント
/// 
/// 現在のターンプレイヤーと、ターン順序を管理します。
/// マルチプレイゲームでのターン制御に使用されます。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TurnManager {
    /// 現在のターンプレイヤーのエンティティID
    pub current_player: Option<Entity>,
    
    /// ターンの順序（エンティティIDのキュー）
    pub turn_order: VecDeque<Entity>,
    
    /// 現在のターン番号（1から開始）
    pub turn_number: u32,
    
    /// ターン開始時刻（UNIXタイムスタンプ）
    pub turn_start_time: u64,
    
    /// ターン制限時間（秒）
    pub turn_time_limit: u32,
}

impl Component for TurnManager {}

impl TurnManager {
    /// 新しいターン管理を作成
    /// 
    /// # 引数
    /// * `players` - プレイヤーエンティティのリスト
    /// * `turn_time_limit` - ターン制限時間（秒）
    /// 
    /// # 戻り値
    /// 初期化されたTurnManagerインスタンス
    pub fn new(players: Vec<Entity>, turn_time_limit: u32) -> Self {
        let turn_order = VecDeque::from(players);
        let current_player = turn_order.front().copied();
        
        Self {
            current_player,
            turn_order,
            turn_number: 1,
            turn_start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            turn_time_limit,
        }
    }
    
    /// 次のプレイヤーにターンを移す
    /// 
    /// # 戻り値
    /// 次のプレイヤーのエンティティID（Noneの場合は全員のターンが終了）
    pub fn next_turn(&mut self) -> Option<Entity> {
        if let Some(current) = self.turn_order.pop_front() {
            // 現在のプレイヤーを末尾に移動（ラウンドロビン）
            self.turn_order.push_back(current);
        }
        
        // 次のプレイヤーを設定
        self.current_player = self.turn_order.front().copied();
        self.turn_number += 1;
        self.turn_start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.current_player
    }
    
    /// 現在のターンの残り時間を取得
    /// 
    /// # 戻り値
    /// 残り時間（秒）。制限なしの場合はNone
    pub fn remaining_time(&self) -> Option<u32> {
        if self.turn_time_limit == 0 {
            return None; // 制限なし
        }
        
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let elapsed = current_time.saturating_sub(self.turn_start_time);
        
        if elapsed >= self.turn_time_limit as u64 {
            Some(0) // 時間切れ
        } else {
            Some(self.turn_time_limit - elapsed as u32)
        }
    }
    
    /// ターンの制限時間が切れているかチェック
    /// 
    /// # 戻り値
    /// 時間切れの場合true、まだ時間がある場合false
    pub fn is_time_up(&self) -> bool {
        if let Some(remaining) = self.remaining_time() {
            remaining == 0
        } else {
            false // 制限なしの場合は常にfalse
        }
    }
    
    /// プレイヤーをターン順序から削除
    /// 
    /// # 引数
    /// * `player` - 削除するプレイヤーのエンティティID
    /// 
    /// # 戻り値
    /// 削除成功時true、失敗時false
    pub fn remove_player(&mut self, player: Entity) -> bool {
        // ターン順序から削除
        let mut found = false;
        self.turn_order.retain(|&p| {
            if p == player {
                found = true;
                false
            } else {
                true
            }
        });
        
        // 現在のプレイヤーが削除された場合、次のプレイヤーに移行
        if self.current_player == Some(player) {
            self.current_player = self.turn_order.front().copied();
        }
        
        found
    }
}

/// ゲームアクション（プレイヤーの行動）を表すコンポーネント
/// 
/// プレイヤーが行った行動を記録し、ゲーム状態の変更や
/// 他のプレイヤーとの同期に使用します。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameAction {
    /// 行動を行ったプレイヤーのエンティティID
    pub player: Entity,
    
    /// 行動の種類
    pub action_type: ActionType,
    
    /// 行動のタイムスタンプ
    pub timestamp: u64,
    
    /// 行動の詳細データ（JSON形式）
    pub data: Option<String>,
}

impl Component for GameAction {}

impl GameAction {
    /// 新しいゲームアクションを作成
    /// 
    /// # 引数
    /// * `player` - 行動を行ったプレイヤー
    /// * `action_type` - 行動の種類
    /// * `data` - 行動の詳細データ（オプション）
    /// 
    /// # 戻り値
    /// 新しいGameActionインスタンス
    pub fn new(player: Entity, action_type: ActionType, data: Option<String>) -> Self {
        Self {
            player,
            action_type,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            data,
        }
    }
}

/// ゲーム内で発生する行動の種類
/// 
/// プレイヤーが実行可能な全ての行動を定義します。
/// 新しい行動を追加する際は、この列挙型に追加してください。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionType {
    /// カードを移動
    MoveCard,
    
    /// カードを裏返す
    FlipCard,
    
    /// カードを引く
    DrawCard,
    
    /// ターンを終了
    EndTurn,
    
    /// ゲームから退出
    LeaveGame,
    
    /// チャットメッセージ送信
    SendMessage,
    
    /// ゲーム設定変更
    ChangeSettings,
}

impl ActionType {
    /// アクション名を文字列で取得
    /// 
    /// # 戻り値
    /// アクション名の文字列
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::MoveCard => "move_card",
            ActionType::FlipCard => "flip_card",
            ActionType::DrawCard => "draw_card",
            ActionType::EndTurn => "end_turn",
            ActionType::LeaveGame => "leave_game",
            ActionType::SendMessage => "send_message",
            ActionType::ChangeSettings => "change_settings",
        }
    }
}

// =============================================================================
// ゲーム状態管理システム群
// =============================================================================

/// ゲーム状態管理システム
/// 
/// ゲーム全体の状態遷移と基本的な管理を行うシステムです。
/// 毎フレーム実行され、ゲームの進行状況をチェックして
/// 必要に応じて状態を更新します。
pub struct GameManagementSystem;

impl System for GameManagementSystem {
    fn update(&mut self, world: &mut World, _delta_time: f64) {
        // すべてのゲーム状態を取得して処理
        let mut phase_changes = Vec::new();
        
        for (entity, game_state) in world.query::<GameState>() {
            match game_state.phase {
                GamePhase::WaitingForPlayers => {
                    // プレイヤー数が十分な場合、開始準備フェーズに移行
                    if game_state.can_start() {
                        phase_changes.push((entity, GamePhase::Starting));
                    }
                },
                
                GamePhase::Starting => {
                    // 開始準備が完了したら、プレイ中フェーズに移行
                    // ここではすぐに移行するが、実際はカード配布などの処理を待つ
                    phase_changes.push((entity, GamePhase::Playing));
                },
                
                GamePhase::Playing => {
                    // ゲーム進行中の処理は他のシステムで管理
                    // ここでは基本的なチェックのみ行う
                },
                
                GamePhase::Paused => {
                    // 一時停止中の特別な処理があればここに記述
                },
                
                GamePhase::Finished | GamePhase::Aborted => {
                    // 終了状態の処理（クリーンアップなど）
                },
            }
        }
        
        // フェーズ変更を適用
        for (entity, new_phase) in phase_changes {
            if let Some(game_state) = world.get_component_mut::<GameState>(entity) {
                if game_state.phase.can_transition_to(new_phase) {
                    game_state.change_phase(new_phase);
                    
                    // フェーズ変更をログ出力
                    println!(
                        "🎮 ゲーム状態変更: {} -> {} (セッション: {})",
                        game_state.phase.as_str(),
                        new_phase.as_str(),
                        game_state.session_id
                    );
                }
            }
        }
    }
}

/// ターン管理システム
/// 
/// プレイヤーのターン制御と時間管理を行うシステムです。
/// ターンの切り替えや制限時間の監視を担当します。
pub struct TurnManagementSystem;

impl System for TurnManagementSystem {
    fn update(&mut self, world: &mut World, _delta_time: f64) {
        let mut turn_changes = Vec::new();
        
        for (entity, turn_manager) in world.query::<TurnManager>() {
            // ターンの制限時間をチェック
            if turn_manager.is_time_up() {
                println!(
                    "⏰ ターン制限時間切れ: プレイヤー {:?} (ターン {})",
                    turn_manager.current_player,
                    turn_manager.turn_number
                );
                turn_changes.push(entity);
            }
            
            // 現在のターン情報をデバッグ出力（制限時間がある場合のみ）
            if let Some(remaining) = turn_manager.remaining_time() {
                if remaining > 0 && remaining % 10 == 0 { // 10秒ごとに表示
                    println!(
                        "⏳ ターン残り時間: {}秒 (プレイヤー: {:?})",
                        remaining,
                        turn_manager.current_player
                    );
                }
            }
        }
        
        // 時間切れのターンを次に進める
        for entity in turn_changes {
            if let Some(turn_manager) = world.get_component_mut::<TurnManager>(entity) {
                let next_player = turn_manager.next_turn();
                println!(
                    "🔄 ターン変更: 次のプレイヤー {:?} (ターン {})",
                    next_player,
                    turn_manager.turn_number
                );
            }
        }
    }
}

/// アクション処理システム
/// 
/// プレイヤーのアクション（行動）を処理し、ゲーム状態に反映するシステムです。
/// アクションの妥当性チェックや副作用の処理を行います。
pub struct ActionProcessingSystem;

impl System for ActionProcessingSystem {
    fn update(&mut self, world: &mut World, _delta_time: f64) {
        let mut processed_actions = Vec::new();
        
        // 全てのアクションを取得して処理
        for (entity, action) in world.query::<GameAction>() {
            println!(
                "🎯 アクション処理: {} by {:?} at {}",
                action.action_type.as_str(),
                action.player,
                action.timestamp
            );
            
            // アクションの種類に応じて処理分岐
            match action.action_type {
                ActionType::MoveCard => {
                    // カード移動の処理
                    // TODO: カードの位置変更ロジックを実装
                },
                
                ActionType::FlipCard => {
                    // カード裏返しの処理
                    // TODO: カードの表裏状態変更ロジックを実装
                },
                
                ActionType::DrawCard => {
                    // カード引きの処理
                    // TODO: デッキからカードを引くロジックを実装
                },
                
                ActionType::EndTurn => {
                    // ターン終了の処理
                    // TODO: ターン管理システムとの連携
                },
                
                ActionType::LeaveGame => {
                    // ゲーム退出の処理
                    // TODO: プレイヤー削除とゲーム状態更新
                },
                
                ActionType::SendMessage => {
                    // チャットメッセージの処理
                    // TODO: メッセージブロードキャスト
                },
                
                ActionType::ChangeSettings => {
                    // 設定変更の処理
                    // TODO: ゲーム設定の更新
                },
            }
            
            // 処理済みアクションとしてマーク
            processed_actions.push(entity);
        }
        
        // 処理済みアクションを削除
        for entity in processed_actions {
            world.remove_component::<GameAction>(entity);
        }
    }
}

// =============================================================================
// ゲーム状態のユーティリティ関数
// =============================================================================

/// ゲームマネージャー
/// 
/// ゲーム状態の管理を支援するユーティリティ構造体です。
/// 複雑なゲーム操作を簡単なメソッド呼び出しで実行できます。
pub struct GameManager;

impl GameManager {
    /// 新しいゲームセッションを作成
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `session_id` - セッションID
    /// * `max_players` - 最大プレイヤー数
    /// 
    /// # 戻り値
    /// 作成されたゲーム状態エンティティ
    pub fn create_game_session(
        world: &mut World,
        session_id: String,
        max_players: u32,
    ) -> Entity {
        let game_entity = world.create_entity();
        let game_state = GameState::new(session_id.clone(), max_players);
        
        world.add_component(game_entity, game_state);
        
        println!("🎮 新しいゲームセッション作成: {} (最大{}人)", session_id, max_players);
        game_entity
    }
    
    /// プレイヤーをゲームに参加させる
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `game_entity` - ゲーム状態エンティティ
    /// * `player_entity` - プレイヤーエンティティ
    /// 
    /// # 戻り値
    /// 参加成功時true、失敗時false
    pub fn join_player(
        world: &mut World,
        game_entity: Entity,
        player_entity: Entity,
    ) -> bool {
        if let Some(game_state) = world.get_component_mut::<GameState>(game_entity) {
            if game_state.add_player() {
                println!("👤 プレイヤー {:?} がゲームに参加しました", player_entity);
                return true;
            }
        }
        false
    }
    
    /// ターン管理を開始
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `game_entity` - ゲーム状態エンティティ  
    /// * `players` - プレイヤーエンティティのリスト
    /// * `turn_time_limit` - ターン制限時間
    /// 
    /// # 戻り値
    /// 作成されたターン管理エンティティ
    pub fn start_turn_management(
        world: &mut World,
        _game_entity: Entity,
        players: Vec<Entity>,
        turn_time_limit: u32,
    ) -> Entity {
        let turn_entity = world.create_entity();
        let turn_manager = TurnManager::new(players.clone(), turn_time_limit);
        
        world.add_component(turn_entity, turn_manager);
        
        println!(
            "🔄 ターン管理開始: {}人のプレイヤー、制限時間{}秒",
            players.len(),
            turn_time_limit
        );
        
        turn_entity
    }
    
    /// プレイヤーアクションを記録
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `player` - アクションを行ったプレイヤー
    /// * `action_type` - アクションの種類
    /// * `data` - アクションの詳細データ
    /// 
    /// # 戻り値
    /// 作成されたアクションエンティティ
    pub fn record_action(
        world: &mut World,
        player: Entity,
        action_type: ActionType,
        data: Option<String>,
    ) -> Entity {
        let action_entity = world.create_entity();
        let game_action = GameAction::new(player, action_type, data);
        
        world.add_component(action_entity, game_action);
        
        println!(
            "📝 アクション記録: {} by {:?}",
            action_type.as_str(),
            player
        );
        
        action_entity
    }
}