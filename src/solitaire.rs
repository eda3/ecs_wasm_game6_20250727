// =============================================================================
// ソリティアゲームロジック
// =============================================================================
// このファイルでは、マルチプレイソリティアゲームのコアロジックを実装します。
// カードの移動ルール、勝利条件の判定、ゲームの進行管理など、
// ソリティアゲーム特有の機能を提供します。
//
// 実装するソリティアの種類：
// - クロンダイク（一般的なソリティア）
// - スパイダー（複数デッキ使用）
// - フリーセル（4つの空きセルを使用）
//
// 主要な責務：
// - カードの有効な移動判定
// - 勝利条件のチェック
// - ゲームの初期設定とカード配布
// - スコア計算とランキング管理
// =============================================================================

use crate::ecs::{World, Entity, Component, System};
use serde::{Serialize, Deserialize};
use std::collections::VecDeque;
// use std::time::{SystemTime, UNIX_EPOCH}; // 未使用のため一時的にコメントアウト

// =============================================================================
// ソリティアゲーム専用のコンポーネント定義
// =============================================================================

/// カードコンポーネント（ソリティア専用の拡張版）
/// 
/// 基本的なカード情報に加えて、ソリティアゲームで必要な
/// 状態情報（位置、可視性、移動可能性など）を管理します。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SolitaireCard {
    /// カードのスート（絵柄）
    pub suit: CardSuit,
    
    /// カードのランク（数値・絵札）
    pub rank: CardRank,
    
    /// カードが表向きかどうか
    pub is_face_up: bool,
    
    /// カードが配置されている場所の種類
    pub location_type: CardLocation,
    
    /// 配置場所内での位置（スタック内の順序など）
    pub position_in_location: u32,
    
    /// カードが移動可能かどうか
    pub is_movable: bool,
    
    /// カードが選択されているかどうか
    pub is_selected: bool,
    
    /// カードの表示座標（アニメーション用）
    pub display_x: f32,
    pub display_y: f32,
    
    /// 移動アニメーションの目標座標
    pub target_x: f32,
    pub target_y: f32,
    
    /// アニメーション中かどうか
    pub is_animating: bool,
}

impl Component for SolitaireCard {}

impl SolitaireCard {
    /// 新しいソリティアカードを作成
    /// 
    /// # 引数
    /// * `suit` - カードのスート
    /// * `rank` - カードのランク
    /// 
    /// # 戻り値
    /// 初期化されたSolitaireCardインスタンス
    pub fn new(suit: CardSuit, rank: CardRank) -> Self {
        Self {
            suit,
            rank,
            is_face_up: false,
            location_type: CardLocation::Deck,
            position_in_location: 0,
            is_movable: false,
            is_selected: false,
            display_x: 0.0,
            display_y: 0.0,
            target_x: 0.0,
            target_y: 0.0,
            is_animating: false,
        }
    }
    
    /// カードを表向きにする
    pub fn flip_up(&mut self) {
        self.is_face_up = true;
        self.update_movability();
    }
    
    /// カードを裏向きにする
    pub fn flip_down(&mut self) {
        self.is_face_up = false;
        self.is_movable = false;
        self.is_selected = false;
    }
    
    /// カードの位置を設定
    /// 
    /// # 引数
    /// * `location_type` - 新しい配置場所
    /// * `position` - 場所内での位置
    pub fn set_location(&mut self, location_type: CardLocation, position: u32) {
        self.location_type = location_type;
        self.position_in_location = position;
        self.update_movability();
    }
    
    /// 表示座標を設定
    /// 
    /// # 引数
    /// * `x` - X座標
    /// * `y` - Y座標
    pub fn set_display_position(&mut self, x: f32, y: f32) {
        self.display_x = x;
        self.display_y = y;
    }
    
    /// アニメーションの目標座標を設定
    /// 
    /// # 引数
    /// * `target_x` - 目標X座標
    /// * `target_y` - 目標Y座標
    pub fn start_animation(&mut self, target_x: f32, target_y: f32) {
        self.target_x = target_x;
        self.target_y = target_y;
        self.is_animating = true;
    }
    
    /// アニメーションを完了
    pub fn finish_animation(&mut self) {
        self.display_x = self.target_x;
        self.display_y = self.target_y;
        self.is_animating = false;
    }
    
    /// カードの移動可能性を更新
    fn update_movability(&mut self) {
        // 表向きのカードのみ移動可能の候補
        if !self.is_face_up {
            self.is_movable = false;
            return;
        }
        
        // 配置場所に応じて移動可能性を決定
        match self.location_type {
            CardLocation::Foundation => {
                // ファウンデーションのカードは通常移動不可
                self.is_movable = false;
            }
            CardLocation::Tableau => {
                // タブローのカードは条件次第で移動可能
                self.is_movable = true;
            }
            CardLocation::Waste => {
                // ウェイストパイルの最上位カードは移動可能
                self.is_movable = true;
            }
            CardLocation::FreeCell => {
                // フリーセルのカードは移動可能
                self.is_movable = true;
            }
            _ => {
                self.is_movable = false;
            }
        }
    }
    
    /// カードの色を取得
    /// 
    /// # 戻り値
    /// カードの色（赤/黒）
    pub fn get_color(&self) -> CardColor {
        match self.suit {
            CardSuit::Hearts | CardSuit::Diamonds => CardColor::Red,
            CardSuit::Clubs | CardSuit::Spades => CardColor::Black,
        }
    }
    
    /// 別のカードの上に置けるかチェック（タブロー用）
    /// Windowsソリティアルール：異なる色で1つ小さいランクのみ配置可能
    /// 
    /// # 引数
    /// * `other` - 下に置かれるカード
    /// 
    /// # 戻り値
    /// 置ける場合true、置けない場合false
    pub fn can_place_on_tableau(&self, other: &SolitaireCard) -> bool {
        // Windowsソリティアの正確なルール
        // 1. 色が異なる必要がある（赤と黒が交互）
        // 2. ランクが1小さい必要がある（例：黒の8の上に赤の7）
        self.get_color() != other.get_color() && 
        (other.rank as u8) == (self.rank as u8) + 1
    }
    
    /// 空のタブロー列に置けるかチェック（Windowsソリティア）
    /// 
    /// # 戻り値
    /// Kingのみ空の列に配置可能
    pub fn can_place_on_empty_tableau(&self) -> bool {
        // Windowsソリティアでは空の列にはKingのみ配置可能
        self.rank == CardRank::King
    }
    
    /// ファウンデーションに置けるかチェック
    /// 
    /// # 引数
    /// * `foundation_top` - ファウンデーションの最上位カード（None の場合は空）
    /// 
    /// # 戻り値
    /// 置ける場合true、置けない場合false
    pub fn can_place_on_foundation(&self, foundation_top: Option<&SolitaireCard>) -> bool {
        match foundation_top {
            None => {
                // 空のファウンデーションにはAceのみ配置可能
                self.rank == CardRank::Ace
            }
            Some(top_card) => {
                // 同じスートで、ランクが1大きい場合のみ配置可能
                self.suit == top_card.suit && 
                (self.rank as u8) == (top_card.rank as u8) + 1
            }
        }
    }
}

/// カードのスート（絵柄）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CardSuit {
    Hearts,   // ♥ ハート
    Diamonds, // ♦ ダイヤ
    Clubs,    // ♣ クラブ  
    Spades,   // ♠ スペード
}

impl CardSuit {
    /// スートの記号を取得
    /// 
    /// # 戻り値
    /// スートの記号文字列
    pub fn symbol(&self) -> &'static str {
        match self {
            CardSuit::Hearts => "♥",
            CardSuit::Diamonds => "♦",
            CardSuit::Clubs => "♣",
            CardSuit::Spades => "♠",
        }
    }
    
    /// 全てのスートを取得
    /// 
    /// # 戻り値
    /// 全スートの配列
    pub fn all() -> [CardSuit; 4] {
        [CardSuit::Hearts, CardSuit::Diamonds, CardSuit::Clubs, CardSuit::Spades]
    }
}

/// カードのランク（数値・絵札）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardRank {
    Ace = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
    Ten = 10,
    Jack = 11,
    Queen = 12,
    King = 13,
}

impl CardRank {
    /// ランクの表示文字列を取得
    /// 
    /// # 戻り値
    /// ランクの表示文字列
    pub fn display(&self) -> &'static str {
        match self {
            CardRank::Ace => "A",
            CardRank::Two => "2",
            CardRank::Three => "3", 
            CardRank::Four => "4",
            CardRank::Five => "5",
            CardRank::Six => "6",
            CardRank::Seven => "7",
            CardRank::Eight => "8",
            CardRank::Nine => "9",
            CardRank::Ten => "10",
            CardRank::Jack => "J",
            CardRank::Queen => "Q",
            CardRank::King => "K",
        }
    }
    
    /// 全てのランクを取得
    /// 
    /// # 戻り値
    /// 全ランクの配列
    pub fn all() -> [CardRank; 13] {
        [
            CardRank::Ace, CardRank::Two, CardRank::Three, CardRank::Four,
            CardRank::Five, CardRank::Six, CardRank::Seven, CardRank::Eight,
            CardRank::Nine, CardRank::Ten, CardRank::Jack, CardRank::Queen, CardRank::King
        ]
    }
}

/// カードの色
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CardColor {
    Red,   // 赤（ハート、ダイヤ）
    Black, // 黒（クラブ、スペード）
}

/// カードの配置場所（Windowsソリティア準拠）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CardLocation {
    /// デッキ（山札）- 左上の裏向きカード置き場
    Deck,
    
    /// ウェイストパイル（捨て札）- デッキの右隣、表向きカード置き場  
    Waste,
    
    /// タブロー（場札、7列）- メインのゲーム盤面
    Tableau,
    
    /// ファウンデーション（組札、4組）- 右上のA〜K完成置き場
    Foundation,
    
    /// フリーセル（空きセル、フリーセル専用）
    FreeCell,
    
    /// 手札（移動中）
    Hand,
}

impl CardLocation {
    /// 場所名を文字列で取得
    /// 
    /// # 戻り値
    /// 場所名の文字列
    pub fn name(&self) -> &'static str {
        match self {
            CardLocation::Deck => "デッキ",
            CardLocation::Waste => "ウェイスト", 
            CardLocation::Tableau => "タブロー",
            CardLocation::Foundation => "ファウンデーション",
            CardLocation::FreeCell => "フリーセル",
            CardLocation::Hand => "手札",
        }
    }
}

/// ゲームタイプ
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SolitaireType {
    /// クロンダイク（通常のソリティア）
    Klondike,
    
    /// スパイダー（2デッキ使用）
    Spider,
    
    /// フリーセル（4つの空きセル）
    FreeCell,
}

impl SolitaireType {
    /// ゲームタイプ名を取得
    /// 
    /// # 戻り値
    /// ゲームタイプ名の文字列
    pub fn name(&self) -> &'static str {
        match self {
            SolitaireType::Klondike => "クロンダイク",
            SolitaireType::Spider => "スパイダー",
            SolitaireType::FreeCell => "フリーセル",
        }
    }
}

/// ソリティアゲーム状態コンポーネント
/// 
/// ゲーム全体の状態（ゲームタイプ、スコア、経過時間など）を管理します。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SolitaireGameState {
    /// ゲームの種類
    pub game_type: SolitaireType,
    
    /// 現在のスコア
    pub score: u32,
    
    /// 移動回数
    pub move_count: u32,
    
    /// ゲーム開始時刻（UNIXタイムスタンプ）
    pub start_time: u64,
    
    /// ゲーム完了フラグ
    pub is_completed: bool,
    
    /// 勝利フラグ
    pub is_won: bool,
    
    /// デッキから引いた回数
    pub deck_turns: u32,
    
    /// ヒントが利用可能かどうか
    pub hint_available: bool,
    
    /// 最後の操作からの経過時間（秒）
    pub idle_time: u64,
}

impl Component for SolitaireGameState {}

impl SolitaireGameState {
    /// 新しいソリティアゲーム状態を作成
    /// 
    /// # 引数
    /// * `game_type` - ゲームの種類
    /// 
    /// # 戻り値
    /// 初期化されたSolitaireGameStateインスタンス
    pub fn new(game_type: SolitaireType) -> Self {
        Self {
            game_type,
            score: 0,
            move_count: 0,
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            is_completed: false,
            is_won: false,
            deck_turns: 0,
            hint_available: true,
            idle_time: 0,
        }
    }
    
    /// 移動を記録してスコアを更新
    /// 
    /// # 引数
    /// * `points` - この移動で獲得するポイント
    pub fn record_move(&mut self, points: u32) {
        self.move_count += 1;
        self.score += points;
        self.idle_time = 0;
        
        // 移動に応じたスコア調整
        match points {
            10 => {
                // ファウンデーションに配置：+10点
            }
            5 => {
                // タブローで表向きにする：+5点
            }
            _ => {
                // その他の移動
            }
        }
        
        println!("📊 移動記録: {}回目, スコア: {}, 獲得ポイント: {}", 
                self.move_count, self.score, points);
    }
    
    /// デッキをめくった回数を記録
    pub fn record_deck_turn(&mut self) {
        self.deck_turns += 1;
        self.idle_time = 0;
        
        // 3回目以降はスコア減点
        if self.deck_turns > 2 {
            if self.score >= 2 {
                self.score -= 2;
            }
        }
        
        println!("🎴 デッキターン: {}回目, スコア: {}", self.deck_turns, self.score);
    }
    
    /// ゲーム完了をチェック
    /// 
    /// # 引数
    /// * `world` - ECSワールド（カードの状態確認用）
    /// 
    /// # 戻り値
    /// ゲームが完了した場合true
    pub fn check_completion(&mut self, world: &World) -> bool {
        if self.is_completed {
            return true;
        }
        
        // ファウンデーションのカード数をチェック
        let mut foundation_count = 0;
        for (_, card) in world.query::<SolitaireCard>() {
            if card.location_type == CardLocation::Foundation {
                foundation_count += 1;
            }
        }
        
        // 全カード（52枚）がファウンデーションに配置されたら勝利
        let required_cards = match self.game_type {
            SolitaireType::Klondike => 52,
            SolitaireType::FreeCell => 52,
            SolitaireType::Spider => 104, // 2デッキ使用
        };
        
        if foundation_count == required_cards {
            self.is_completed = true;
            self.is_won = true;
            self.calculate_final_score();
            
            println!("🎉 ゲーム完了！勝利！最終スコア: {}", self.score);
            return true;
        }
        
        false
    }
    
    /// 最終スコアを計算
    fn calculate_final_score(&mut self) {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let elapsed_time = current_time - self.start_time;
        
        // 時間ボーナス（早いほど高得点）
        let time_bonus = if elapsed_time < 300 { // 5分以内
            100
        } else if elapsed_time < 600 { // 10分以内
            50
        } else {
            0
        };
        
        // 移動回数ペナルティ（少ないほど高得点）
        let move_penalty = if self.move_count > 200 {
            20
        } else if self.move_count > 100 {
            10
        } else {
            0
        };
        
        self.score = self.score.saturating_add(time_bonus).saturating_sub(move_penalty);
        
        println!("⭐ 最終スコア計算:");
        println!("  基本スコア: {}", self.score - time_bonus + move_penalty);
        println!("  時間ボーナス: +{}", time_bonus);
        println!("  移動ペナルティ: -{}", move_penalty);
        println!("  最終スコア: {}", self.score);
    }
    
    /// 経過時間を更新
    /// 
    /// # 引数
    /// * `delta_time` - フレーム間の経過時間（秒）
    pub fn update_idle_time(&mut self, delta_time: f64) {
        self.idle_time += delta_time as u64;
    }
}

/// カードスタック（複数カードの管理）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardStack {
    /// スタック内のカードエンティティ
    pub cards: VecDeque<Entity>,
    
    /// スタックの種類
    pub stack_type: CardLocation,
    
    /// スタックのインデックス（タブローの列番号など）
    pub stack_index: u32,
    
    /// スタックの基準座標
    pub base_x: f32,
    pub base_y: f32,
    
    /// カード間の間隔
    pub card_spacing: f32,
    
    /// 最大容量（-1 = 無制限）
    pub max_capacity: i32,
}

impl Component for CardStack {}

impl CardStack {
    /// 新しいカードスタックを作成
    /// 
    /// # 引数
    /// * `stack_type` - スタックの種類
    /// * `stack_index` - スタックのインデックス
    /// * `base_x` - 基準X座標
    /// * `base_y` - 基準Y座標
    /// 
    /// # 戻り値
    /// 初期化されたCardStackインスタンス
    pub fn new(
        stack_type: CardLocation,
        stack_index: u32,
        base_x: f32,
        base_y: f32,
    ) -> Self {
        let (card_spacing, max_capacity) = match stack_type {
            CardLocation::Tableau => (20.0, -1), // 無制限、20pxずつずらす
            CardLocation::Foundation => (0.0, 13), // カード13枚、重ねて配置
            CardLocation::FreeCell => (0.0, 1), // 1枚のみ
            CardLocation::Waste => (0.0, -1), // 無制限、重ねて配置
            _ => (0.0, -1),
        };
        
        Self {
            cards: VecDeque::new(),
            stack_type,
            stack_index,
            base_x,
            base_y,
            card_spacing,
            max_capacity,
        }
    }
    
    /// カードをスタックの最上部に追加
    /// 
    /// # 引数
    /// * `card_entity` - 追加するカードエンティティ
    /// 
    /// # 戻り値
    /// 追加成功時true、失敗時false
    pub fn push_card(&mut self, card_entity: Entity) -> bool {
        if self.max_capacity > 0 && self.cards.len() >= self.max_capacity as usize {
            return false; // 容量オーバー
        }
        
        self.cards.push_back(card_entity);
        true
    }
    
    /// スタックの最上部からカードを取り出し
    /// 
    /// # 戻り値
    /// 取り出したカードエンティティ、空の場合はNone
    pub fn pop_card(&mut self) -> Option<Entity> {
        self.cards.pop_back()
    }
    
    /// スタックの最上部のカードを取得（取り出さない）
    /// 
    /// # 戻り値
    /// 最上部のカードエンティティ、空の場合はNone
    pub fn peek_top(&self) -> Option<Entity> {
        self.cards.back().copied()
    }
    
    /// スタックが空かどうかチェック
    /// 
    /// # 戻り値
    /// 空の場合true
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
    
    /// スタック内のカード数を取得
    /// 
    /// # 戻り値
    /// カード数
    pub fn len(&self) -> usize {
        self.cards.len()
    }
    
    /// 指定位置のカードの表示座標を計算
    /// 
    /// # 引数
    /// * `position` - スタック内の位置（0が最下部）
    /// 
    /// # 戻り値
    /// (x, y) 座標のタプル
    pub fn calculate_card_position(&self, position: usize) -> (f32, f32) {
        match self.stack_type {
            CardLocation::Tableau => {
                // タブローでは下向きに重ねる
                (self.base_x, self.base_y + (position as f32 * self.card_spacing))
            }
            _ => {
                // その他では同じ位置に重ねる
                (self.base_x, self.base_y)
            }
        }
    }
}

// =============================================================================
// ソリティアゲーム管理システム群
// =============================================================================

/// カード移動システム
/// 
/// カードの移動ルールをチェックし、有効な移動を実行するシステムです。
pub struct CardMovementSystem;

impl System for CardMovementSystem {
    fn update(&mut self, world: &mut World, _delta_time: f64) {
        // 選択されたカードを検索
        let mut selected_entities = Vec::new();
        for (entity, card) in world.query::<SolitaireCard>() {
            if card.is_selected {
                selected_entities.push((entity, card.suit, card.rank, card.location_type));
            }
        }
        
        if selected_entities.is_empty() {
            return;
        }
        
        // 選択されたカードの移動処理
        for (entity, suit, rank, location_type) in selected_entities {
            println!("🎯 選択されたカード: {}{} ({})", 
                    suit.symbol(), rank.display(), location_type.name());
            
            // TODO: マウス/タッチ入力に基づく移動先の決定
            // TODO: 移動ルールの検証
            // TODO: 移動の実行
            
            // 一時的に選択解除（実際の実装では移動完了時に解除）
            if let Some(card_mut) = world.get_component_mut::<SolitaireCard>(entity) {
                card_mut.is_selected = false;
            }
        }
    }
}

/// カードアニメーションシステム
/// 
/// カードの移動アニメーションを管理するシステムです。
pub struct CardAnimationSystem;

impl System for CardAnimationSystem {
    fn update(&mut self, world: &mut World, delta_time: f64) {
        let animation_speed = 500.0; // ピクセル/秒
        let mut animating_cards = Vec::new();
        let mut completed_animations = Vec::new();
        
        // アニメーション中のカードを特定
        for (entity, card) in world.query::<SolitaireCard>() {
            if card.is_animating {
                let dx = card.target_x - card.display_x;
                let dy = card.target_y - card.display_y;
                let distance = (dx * dx + dy * dy).sqrt();
                
                if distance < 2.0 {
                    completed_animations.push(entity);
                } else {
                    let move_distance = animation_speed * delta_time as f32;
                    let move_ratio = move_distance / distance;
                    animating_cards.push((entity, dx * move_ratio, dy * move_ratio));
                }
            }
        }
        
        // アニメーションを更新
        for (entity, dx, dy) in animating_cards {
            if let Some(card_mut) = world.get_component_mut::<SolitaireCard>(entity) {
                card_mut.display_x += dx;
                card_mut.display_y += dy;
            }
        }
        
        // アニメーション完了処理
        for entity in completed_animations {
            if let Some(card) = world.get_component_mut::<SolitaireCard>(entity) {
                let suit_symbol = card.suit.symbol();
                let rank_display = card.rank.display();
                card.finish_animation();
                println!("✨ カードアニメーション完了: {}{}", suit_symbol, rank_display);
            }
        }
    }
}

/// ゲーム進行管理システム
/// 
/// ソリティアゲームの進行状況を監視し、勝利条件などをチェックします。
pub struct SolitaireProgressSystem;

impl System for SolitaireProgressSystem {
    fn update(&mut self, world: &mut World, delta_time: f64) {
        let mut game_completed = false;
        
        // ゲーム状態を取得して更新
        let mut game_entities = Vec::new();
        for (entity, game_state) in world.query::<SolitaireGameState>() {
            if !game_state.is_completed {
                game_entities.push(entity);
            }
        }
        
        for entity in game_entities {
            if let Some(game_state_mut) = world.get_component_mut::<SolitaireGameState>(entity) {
                // アイドル時間を更新
                game_state_mut.update_idle_time(delta_time);
                
                // 勝利条件をチェック（borrowingの競合を避けるため、分離して処理）
                let temp_completed = game_state_mut.is_completed;
                if temp_completed {
                    game_completed = true;
                } else {
                    // mutable borrowを一時的に解除してからカード数をチェック
                    drop(game_state_mut);
                    
                    // ファウンデーションのカード数をチェック
                    let mut foundation_count = 0;
                    for (_, card) in world.query::<SolitaireCard>() {
                        if matches!(card.location_type, CardLocation::Foundation) {
                            foundation_count += 1;
                        }
                    }
                    
                    // 52枚全てがファウンデーションにあれば完了
                    if foundation_count >= 52 {
                        if let Some(game_state_mut) = world.get_component_mut::<SolitaireGameState>(entity) {
                            game_state_mut.is_completed = true;
                            game_state_mut.is_won = true;
                        }
                        game_completed = true;
                    }
                }
                
                // 長時間アイドル時のヒント表示（再度borrowする）
                if let Some(game_state_mut) = world.get_component_mut::<SolitaireGameState>(entity) {
                    if game_state_mut.idle_time > 30 && game_state_mut.hint_available {
                        println!("💡 ヒント: 移動可能なカードを探してみてください");
                        game_state_mut.hint_available = false;
                    }
                }
            }
        }
        
        if game_completed {
            println!("🏆 ゲーム完了！おめでとうございます！");
        }
    }
}

// =============================================================================
// ソリティアゲーム管理のユーティリティ関数
// =============================================================================

/// ソリティアゲーム管理マネージャー
/// 
/// ソリティアゲームの初期化、カード配布、ルール管理を行います。
pub struct SolitaireManager;

impl SolitaireManager {
    /// 新しいソリティアゲームを開始
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `game_type` - ゲームの種類
    /// 
    /// # 戻り値
    /// ゲーム状態エンティティ
    pub fn start_new_game(
        world: &mut World,
        game_type: SolitaireType,
    ) -> Entity {
        println!("🎮 新しい{}ゲームを開始します", game_type.name());
        
        // ゲーム状態を作成
        let game_entity = world.create_entity();
        let game_state = SolitaireGameState::new(game_type);
        world.add_component(game_entity, game_state);
        
        // カードデッキを作成・配布
        let cards = Self::create_deck(world, game_type);
        Self::deal_cards(world, game_type, cards);
        
        // カードスタックを作成
        Self::create_stacks(world, game_type);
        
        println!("✅ ゲーム初期化完了");
        game_entity
    }
    
    /// カードデッキを作成
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `game_type` - ゲームの種類
    /// 
    /// # 戻り値
    /// 作成されたカードエンティティのベクター
    fn create_deck(world: &mut World, game_type: SolitaireType) -> Vec<Entity> {
        let mut cards = Vec::new();
        let deck_count = match game_type {
            SolitaireType::Spider => 2, // スパイダーは2デッキ
            _ => 1,
        };
        
        for _ in 0..deck_count {
            for suit in CardSuit::all() {
                for rank in CardRank::all() {
                    let card_entity = world.create_entity();
                    let card = SolitaireCard::new(suit, rank);
                    world.add_component(card_entity, card);
                    cards.push(card_entity);
                }
            }
        }
        
        // カードをシャッフル（簡単な実装）
        Self::shuffle_cards(&mut cards);
        
        println!("🎴 {}デッキ作成完了: {}枚", deck_count, cards.len());
        cards
    }
    
    /// カードをシャッフル
    /// 
    /// # 引数
    /// * `cards` - シャッフルするカードエンティティのベクター
    fn shuffle_cards(cards: &mut Vec<Entity>) {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as usize;
        
        // 簡単なシャッフルアルゴリズム
        for i in (1..cards.len()).rev() {
            let j = (seed * (i + 1) * 31) % (i + 1);
            cards.swap(i, j);
        }
    }
    
    /// カードを配布
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `game_type` - ゲームの種類
    /// * `cards` - 配布するカードエンティティのベクター
    fn deal_cards(world: &mut World, game_type: SolitaireType, mut cards: Vec<Entity>) {
        match game_type {
            SolitaireType::Klondike => {
                Self::deal_klondike(world, &mut cards);
            }
            SolitaireType::FreeCell => {
                Self::deal_freecell(world, &mut cards);
            }
            SolitaireType::Spider => {
                Self::deal_spider(world, &mut cards);
            }
        }
    }
    
    /// クロンダイク用のカード配布（Windowsソリティア標準）
    /// 
    /// Windowsソリティアの正確なレイアウト：
    /// - タブロー: 7列、左から1,2,3,4,5,6,7枚
    /// - 各列の最上位カードのみ表向き
    /// - ファウンデーション: 4つの組札（A〜K順）
    /// - デッキ: 残り24枚（裏向き）
    /// - ウェイスト: デッキから引いたカード（表向き）
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `cards` - 配布するカードエンティティのベクター
    fn deal_klondike(world: &mut World, cards: &mut Vec<Entity>) {
        let mut card_index = 0;
        
        // タブローに配布（7列、各列に1〜7枚）
        // Windowsソリティアの標準配置
        for column in 0..7 {
            for row in 0..=column {
                if card_index >= cards.len() {
                    break;
                }
                
                let card_entity = cards[card_index];
                if let Some(card) = world.get_component_mut::<SolitaireCard>(card_entity) {
                    card.set_location(CardLocation::Tableau, column);
                    
                    // Windowsソリティアの正確な配置座標
                    let base_x = 20.0 + column as f32 * 100.0; // 左端から20px、間隔100px
                    let base_y = 150.0 + row as f32 * 25.0;   // 上から150px、重なり25px
                    card.set_display_position(base_x, base_y);
                    
                    // 各列の最上位カードのみ表向き（Windowsソリティアルール）
                    if row == column {
                        card.flip_up();
                        card.is_movable = true;
                    } else {
                        card.flip_down();
                        card.is_movable = false;
                    }
                }
                
                card_index += 1;
            }
        }
        
        // 残りのカードはデッキに（Windowsソリティアでは24枚）
        for i in card_index..cards.len() {
            let card_entity = cards[i];
            if let Some(card) = world.get_component_mut::<SolitaireCard>(card_entity) {
                card.set_location(CardLocation::Deck, i as u32 - card_index as u32);
                card.set_display_position(20.0, 20.0); // 左上のデッキ位置
                card.flip_down(); // デッキのカードは裏向き
                card.is_movable = false;
            }
        }
        
        println!("📋 Windowsクロンダイク配布完了: タブロー{}枚, デッキ{}枚", 
                card_index, cards.len() - card_index);
        
        // 配置詳細をログ出力
        println!("  タブロー配置:");
        for i in 0..7 {
            println!("    列{}: {}枚（最上位のみ表向き）", i + 1, i + 1);
        }
        println!("  デッキ: 24枚（全て裏向き）");
        println!("  ファウンデーション: 4つの空スペース（A〜K順に積む）");
    }
    
    /// フリーセル用のカード配布
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `cards` - 配布するカードエンティティのベクター
    fn deal_freecell(world: &mut World, cards: &mut Vec<Entity>) {
        // 8列に均等配布（各列6-7枚）
        for (i, card_entity) in cards.iter().enumerate() {
            let column = i % 8;
            let row = i / 8;
            
            if let Some(card) = world.get_component_mut::<SolitaireCard>(*card_entity) {
                card.set_location(CardLocation::Tableau, column as u32);
                card.set_display_position(50.0 + column as f32 * 100.0, 200.0 + row as f32 * 20.0);
                card.flip_up(); // フリーセルでは全カード表向き
            }
        }
        
        println!("📋 フリーセル配布完了: 8列に52枚配布");
    }
    
    /// スパイダー用のカード配布
    /// 
    /// # 引数  
    /// * `world` - ECSワールドへの可変参照
    /// * `cards` - 配布するカードエンティティのベクター
    fn deal_spider(world: &mut World, cards: &mut Vec<Entity>) {
        let mut card_index = 0;
        
        // 10列に配布（各列5-6枚）
        for column in 0..10 {
            let cards_in_column = if column < 4 { 6 } else { 5 };
            
            for row in 0..cards_in_column {
                if card_index >= cards.len() {
                    break;
                }
                
                let card_entity = cards[card_index];
                if let Some(card) = world.get_component_mut::<SolitaireCard>(card_entity) {
                    card.set_location(CardLocation::Tableau, column);
                    card.set_display_position(50.0 + column as f32 * 80.0, 200.0 + row as f32 * 15.0);
                    
                    // 各列の最上位カードのみ表向き
                    if row == cards_in_column - 1 {
                        card.flip_up();
                    }
                }
                
                card_index += 1;
            }
        }
        
        // 残りのカードはデッキに
        for i in card_index..cards.len() {
            let card_entity = cards[i];
            if let Some(card) = world.get_component_mut::<SolitaireCard>(card_entity) {
                card.set_location(CardLocation::Deck, 0);
                card.set_display_position(50.0, 100.0);
            }
        }
        
        println!("📋 スパイダー配布完了: タブロー{}枚, デッキ{}枚", 
                card_index, cards.len() - card_index);
    }
    
    /// カードスタックを作成
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `game_type` - ゲームの種類
    fn create_stacks(world: &mut World, game_type: SolitaireType) {
        match game_type {
            SolitaireType::Klondike => {
                // タブロー（7列）
                for i in 0..7 {
                    let stack_entity = world.create_entity();
                    let stack = CardStack::new(
                        CardLocation::Tableau,
                        i,
                        100.0 + i as f32 * 120.0,
                        200.0,
                    );
                    world.add_component(stack_entity, stack);
                }
                
                // ファウンデーション（4組）
                for i in 0..4 {
                    let stack_entity = world.create_entity();
                    let stack = CardStack::new(
                        CardLocation::Foundation,
                        i,
                        400.0 + i as f32 * 120.0,
                        50.0,
                    );
                    world.add_component(stack_entity, stack);
                }
            }
            
            SolitaireType::FreeCell => {
                // タブロー（8列）
                for i in 0..8 {
                    let stack_entity = world.create_entity();
                    let stack = CardStack::new(
                        CardLocation::Tableau,
                        i,
                        50.0 + i as f32 * 100.0,
                        200.0,
                    );
                    world.add_component(stack_entity, stack);
                }
                
                // フリーセル（4つ）
                for i in 0..4 {
                    let stack_entity = world.create_entity();
                    let stack = CardStack::new(
                        CardLocation::FreeCell,
                        i,
                        50.0 + i as f32 * 100.0,
                        50.0,
                    );
                    world.add_component(stack_entity, stack);
                }
                
                // ファウンデーション（4組）
                for i in 0..4 {
                    let stack_entity = world.create_entity();
                    let stack = CardStack::new(
                        CardLocation::Foundation,
                        i,
                        450.0 + i as f32 * 100.0,
                        50.0,
                    );
                    world.add_component(stack_entity, stack);
                }
            }
            
            SolitaireType::Spider => {
                // タブロー（10列）
                for i in 0..10 {
                    let stack_entity = world.create_entity();
                    let stack = CardStack::new(
                        CardLocation::Tableau,
                        i,
                        50.0 + i as f32 * 80.0,
                        200.0,
                    );
                    world.add_component(stack_entity, stack);
                }
                
                // ファウンデーション（8組、2デッキ分）
                for i in 0..8 {
                    let stack_entity = world.create_entity();
                    let stack = CardStack::new(
                        CardLocation::Foundation,
                        i,
                        50.0 + i as f32 * 80.0,
                        50.0,
                    );
                    world.add_component(stack_entity, stack);
                }
            }
        }
        
        println!("📚 {}用スタック作成完了", game_type.name());
    }
    
    /// Windowsソリティア専用：デッキからカードを引く
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// 
    /// # 戻り値
    /// カードを引けた場合true、デッキが空の場合false
    pub fn draw_from_deck(world: &mut World) -> bool {
        // デッキのカードを探す
        let mut deck_cards = Vec::new();
        for (entity, card) in world.query::<SolitaireCard>() {
            if card.location_type == CardLocation::Deck {
                deck_cards.push((entity, card.position_in_location));
            }
        }
        
        if deck_cards.is_empty() {
            // デッキが空の場合、ウェイストパイルのカードをデッキに戻す
            return Self::recycle_waste_to_deck(world);
        }
        
        // 最上位のカード（position_in_location最大）を取得
        deck_cards.sort_by_key(|(_, pos)| *pos);
        if let Some((card_entity, _)) = deck_cards.last() {
            if let Some(card) = world.get_component_mut::<SolitaireCard>(*card_entity) {
                // ウェイストパイルに移動
                card.set_location(CardLocation::Waste, 0);
                card.set_display_position(140.0, 20.0); // デッキの右隣
                card.flip_up();
                card.is_movable = true;
                
                println!("🎴 デッキからカードを引きました: {}{}", 
                        card.suit.symbol(), card.rank.display());
                return true;
            }
        }
        
        false
    }
    
    /// Windowsソリティア専用：ウェイストパイルをデッキに戻す
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// 
    /// # 戻り値
    /// カードを戻せた場合true、ウェイストも空の場合false
    fn recycle_waste_to_deck(world: &mut World) -> bool {
        let mut waste_cards = Vec::new();
        for (entity, _card) in world.query::<SolitaireCard>() {
            if _card.location_type == CardLocation::Waste {
                waste_cards.push(entity);
            }
        }
        
        if waste_cards.is_empty() {
            println!("⚠️ デッキもウェイストも空です");
            return false;
        }
        
        println!("♻️ ウェイストパイルをデッキに戻します（{}枚）", waste_cards.len());
        
        // ウェイストのカードを逆順でデッキに戻す（Windowsソリティアの仕様）
        for (i, card_entity) in waste_cards.iter().rev().enumerate() {
            if let Some(card) = world.get_component_mut::<SolitaireCard>(*card_entity) {
                card.set_location(CardLocation::Deck, i as u32);
                card.set_display_position(20.0, 20.0);
                card.flip_down();
                card.is_movable = false;
            }
        }
        
        true
    }
    
    /// Windowsソリティア専用：カードの自動配置（ダブルクリック時）
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `card_entity` - 自動配置するカードエンティティ
    /// 
    /// # 戻り値
    /// 配置できた場合true、できない場合false
    pub fn auto_place_card(world: &mut World, card_entity: Entity) -> bool {
        // カード情報を取得（コピーして借用競合を回避）
        let card_copy = if let Some(card) = world.get_component::<SolitaireCard>(card_entity) {
            card.clone()
        } else {
            return false;
        };
        
        // まずファウンデーションに配置を試行
        if Self::try_place_on_foundation(world, card_entity, &card_copy) {
            return true;
        }
        
        // ファウンデーションに配置できない場合、タブローを試行
        if Self::try_place_on_tableau(world, card_entity, &card_copy) {
            return true;
        }
        
        false
    }
    
    /// ファウンデーションへの配置を試行
    fn try_place_on_foundation(world: &mut World, card_entity: Entity, card: &SolitaireCard) -> bool {
        // 各ファウンデーションをチェック
        for foundation_index in 0..4 {
            // 該当するファウンデーションの最上位カードを取得
            let foundation_top = Self::get_foundation_top(world, foundation_index);
            
            if card.can_place_on_foundation(foundation_top.as_ref()) {
                if let Some(card_mut) = world.get_component_mut::<SolitaireCard>(card_entity) {
                    let foundation_x = 400.0 + foundation_index as f32 * 100.0;
                    card_mut.set_location(CardLocation::Foundation, foundation_index);
                    card_mut.set_display_position(foundation_x, 20.0);
                    
                    println!("✨ ファウンデーション{}に自動配置: {}{}", 
                            foundation_index + 1, card.suit.symbol(), card.rank.display());
                    return true;
                }
            }
        }
        false
    }
    
    /// タブローへの配置を試行
    fn try_place_on_tableau(world: &mut World, card_entity: Entity, card: &SolitaireCard) -> bool {
        // 各タブロー列をチェック
        for column in 0..7 {
            let tableau_top = Self::get_tableau_top(world, column);
            
            let can_place = match tableau_top {
                Some(top_card) => card.can_place_on_tableau(&top_card),
                None => card.can_place_on_empty_tableau(),
            };
            
            if can_place {
                // カード数を先に計算（借用競合を回避）
                let card_count = Self::count_tableau_cards(world, column);
                
                if let Some(card_mut) = world.get_component_mut::<SolitaireCard>(card_entity) {
                    let column_x = 20.0 + column as f32 * 100.0;
                    let column_y = 150.0 + card_count as f32 * 25.0;
                    
                    card_mut.set_location(CardLocation::Tableau, column);
                    card_mut.set_display_position(column_x, column_y);
                    
                    println!("✨ タブロー列{}に自動配置: {}{}", 
                            column + 1, card.suit.symbol(), card.rank.display());
                    return true;
                }
            }
        }
        false
    }
    
    /// ファウンデーションの最上位カードを取得
    fn get_foundation_top(world: &World, foundation_index: u32) -> Option<SolitaireCard> {
        let mut foundation_cards = Vec::new();
        for (_entity, card) in world.query::<SolitaireCard>() {
            if card.location_type == CardLocation::Foundation && 
               card.position_in_location == foundation_index {
                foundation_cards.push(card.clone());
            }
        }
        
        // 最新のカード（最も高いランク）を取得
        foundation_cards.into_iter()
            .max_by_key(|card| card.rank as u8)
    }
    
    /// タブロー列の最上位カードを取得
    fn get_tableau_top(world: &World, column: u32) -> Option<SolitaireCard> {
        let mut column_cards = Vec::new();
        for (_entity, card) in world.query::<SolitaireCard>() {
            if card.location_type == CardLocation::Tableau && 
               card.position_in_location == column {
                column_cards.push(card.clone());
            }
        }
        
        // 最上位のカード（表向きで最も下にある）を取得
        column_cards.into_iter()
            .filter(|card| card.is_face_up)
            .max_by_key(|card| card.display_y as i32)
    }
    
    /// タブロー列のカード数をカウント
    fn count_tableau_cards(world: &World, column: u32) -> usize {
        world.query::<SolitaireCard>()
            .filter(|(_entity, card)| {
                card.location_type == CardLocation::Tableau && 
                card.position_in_location == column
            })
            .count()
    }
    
    /// Windowsソリティア専用：勝利条件チェック
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// 
    /// # 戻り値
    /// 勝利している場合true
    pub fn check_windows_solitaire_win(world: &World) -> bool {
        // 4つのファウンデーションすべてにKingが配置されているかチェック
        let mut completed_foundations = 0;
        
        for foundation_index in 0..4 {
            if let Some(top_card) = Self::get_foundation_top(world, foundation_index) {
                if top_card.rank == CardRank::King {
                    completed_foundations += 1;
                }
            }
        }
        
        if completed_foundations == 4 {
            println!("🎉 おめでとうございます！Windowsソリティアをクリアしました！");
            return true;
        }
        
        false
    }
}