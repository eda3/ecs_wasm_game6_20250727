// =============================================================================
// ECS WASM ソリティアゲーム - 開発・テスト用メインエントリーポイント
// =============================================================================
// このファイルは`cargo run`コマンドで実行するためのバイナリクレートです。
// WebAssembly版ではsrc/lib.rsが使用されますが、開発中の動作確認や
// テストのために、ネイティブRustとして実行できるようにしています。
//
// 用途：
// - ECSシステムの動作確認
// - ゲームロジックのテスト
// - パフォーマンス測定
// - デバッグ作業
// =============================================================================

// 自作ECSシステムをインポート
mod ecs;

use ecs::{World, Entity, Component, System, SystemScheduler};
use std::time::{Duration, Instant};

// =============================================================================
// ソリティアゲーム用のコンポーネント定義
// =============================================================================

/// 位置コンポーネント
/// エンティティの2D座標を表します
#[derive(Debug, Clone, Copy, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

impl Component for Position {}

impl Position {
    /// 新しい位置コンポーネントを作成
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// カードコンポーネント
/// トランプカードの情報を表します
#[derive(Debug, Clone, PartialEq)]
struct Card {
    suit: Suit,    // スート（ハート、スペードなど）
    rank: Rank,    // ランク（A、2-10、J、Q、K）
    is_face_up: bool,  // 表向きかどうか
}

impl Component for Card {}

/// トランプのスート（絵柄）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Suit {
    Hearts,   // ♥ ハート
    Diamonds, // ♦ ダイヤ
    Clubs,    // ♣ クラブ
    Spades,   // ♠ スペード
}

/// トランプのランク（数字・絵札）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Rank {
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

impl Card {
    /// 新しいカードを作成
    pub fn new(suit: Suit, rank: Rank) -> Self {
        Self {
            suit,
            rank,
            is_face_up: false,
        }
    }

    /// カードを表向きにする
    pub fn flip_up(&mut self) {
        self.is_face_up = true;
    }

    /// カードを裏向きにする  
    pub fn flip_down(&mut self) {
        self.is_face_up = false;
    }
}

/// プレイヤー情報コンポーネント
#[derive(Debug, Clone, PartialEq)]
struct Player {
    name: String,
    score: u32,
    is_connected: bool,
}

impl Component for Player {}

impl Player {
    /// 新しいプレイヤーを作成
    pub fn new(name: String) -> Self {
        Self {
            name,
            score: 0,
            is_connected: true,
        }
    }
}

// =============================================================================
// ソリティアゲーム用のシステム定義
// =============================================================================

/// カード描画システム
/// 全てのカードの位置と状態を表示するシステム
struct CardRenderSystem;

impl System for CardRenderSystem {
    fn update(&mut self, world: &mut World, _delta_time: f64) {
        println!("🎴 === カード描画システム実行 ===");
        
        // カードコンポーネントを持つ全エンティティを検索
        let mut card_count = 0;
        for (entity, card) in world.query::<Card>() {
            card_count += 1;
            
            // 位置情報も取得（オプション）
            let position_info = if let Some(pos) = world.get_component::<Position>(entity) {
                format!("位置: ({:.1}, {:.1})", pos.x, pos.y)
            } else {
                "位置: 未設定".to_string()
            };
            
            // カード情報を表示
            let face_status = if card.is_face_up { "表" } else { "裏" };
            println!(
                "  エンティティ{}: {:?} の {:?} [{}] - {}",
                entity.id(),
                card.suit,
                card.rank,
                face_status,
                position_info
            );
        }
        
        if card_count > 0 {
            println!("  合計: {}枚のカード", card_count);
        } else {
            println!("  カードが見つかりません");
        }
        println!();
    }
}

/// プレイヤー情報システム
/// プレイヤーの状態を表示・管理するシステム
struct PlayerSystem;

impl System for PlayerSystem {
    fn update(&mut self, world: &mut World, _delta_time: f64) {
        println!("👤 === プレイヤー情報システム実行 ===");
        
        let mut player_count = 0;
        for (entity, player) in world.query::<Player>() {
            player_count += 1;
            
            let connection_status = if player.is_connected { "接続中" } else { "切断" };
            println!(
                "  エンティティ{}: プレイヤー「{}」 - スコア: {} - 状態: {}",
                entity.id(),
                player.name,
                player.score,
                connection_status
            );
        }
        
        if player_count > 0 {
            println!("  合計: {}人のプレイヤー", player_count);
        } else {
            println!("  プレイヤーが見つかりません");
        }
        println!();
    }
}

// =============================================================================
// テスト用のゲームデータ作成関数
// =============================================================================

/// テスト用のカードデッキを作成
fn create_test_deck(world: &mut World) -> Vec<Entity> {
    println!("🃏 テスト用カードデッキを作成中...");
    
    let suits = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
    let ranks = [
        Rank::Ace, Rank::Two, Rank::Three, Rank::Four, Rank::Five,
        Rank::Six, Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten,
        Rank::Jack, Rank::Queen, Rank::King
    ];
    
    let mut deck = Vec::new();
    let mut x_pos = 0.0;
    let mut y_pos = 0.0;
    
    // 全てのカードを作成（52枚）
    for suit in &suits {
        for rank in &ranks {
            let entity = world.create_entity();
            
            // カードコンポーネントを追加
            let mut card = Card::new(*suit, *rank);
            // 最初の数枚は表向きにする（テスト用）
            if deck.len() < 5 {
                card.flip_up();
            }
            world.add_component(entity, card);
            
            // 位置コンポーネントを追加
            world.add_component(entity, Position::new(x_pos, y_pos));
            
            deck.push(entity);
            
            // 位置を次のカード用に調整
            x_pos += 50.0;
            if x_pos > 300.0 {
                x_pos = 0.0;
                y_pos += 70.0;
            }
        }
    }
    
    println!("  作成完了: {}枚のカード", deck.len());
    deck
}

/// テスト用のプレイヤーを作成
fn create_test_players(world: &mut World) -> Vec<Entity> {
    println!("👥 テスト用プレイヤーを作成中...");
    
    let player_names = ["Alice", "Bob", "Carol"];
    let mut players = Vec::new();
    
    for (i, name) in player_names.iter().enumerate() {
        let entity = world.create_entity();
        
        // プレイヤーコンポーネントを追加
        let mut player = Player::new(name.to_string());
        player.score = ((i + 1) * 100) as u32; // テスト用スコア
        world.add_component(entity, player);
        
        // プレイヤーの位置を設定
        world.add_component(entity, Position::new(i as f32 * 100.0, -50.0));
        
        players.push(entity);
    }
    
    println!("  作成完了: {}人のプレイヤー", players.len());
    players
}

// =============================================================================
// メイン関数
// =============================================================================

fn main() {
    println!("🎮 ECS WASM ソリティアゲーム - 開発モード開始！");
    println!("{}", "=".repeat(50));
    
    // ECSワールドを初期化
    let mut world = World::new();
    println!("✅ ECSワールドを初期化しました");
    
    // システムスケジューラを作成
    let mut scheduler = SystemScheduler::new();
    scheduler.add_system(CardRenderSystem);
    scheduler.add_system(PlayerSystem);
    println!("✅ システムスケジューラを初期化しました");
    
    // テストデータを作成
    println!("\n📦 テストデータ作成中...");
    let _deck = create_test_deck(&mut world);
    let _players = create_test_players(&mut world);
    
    println!("\n📊 ワールド状態:");
    println!("  エンティティ総数: {}", world.entity_count());
    
    // システムを数回実行してテスト
    println!("\n🔄 ECSシステム実行テスト開始...");
    
    for frame in 1..=3 {
        println!("--- フレーム {} ---", frame);
        let start_time = Instant::now();
        
        scheduler.update(&mut world, 16.67); // 60FPS想定のデルタタイム
        
        let elapsed = start_time.elapsed();
        println!("実行時間: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
        
        // フレーム間隔をシミュレート
        std::thread::sleep(Duration::from_millis(100));
    }
    
    // 動的なコンポーネント操作のテスト
    println!("🔧 動的操作テスト...");
    if let Some(first_player) = _players.first() {
        if let Some(player) = world.get_component_mut::<Player>(*first_player) {
            player.score += 500;
            println!("  プレイヤー「{}」のスコアを更新しました", player.name);
        }
    }
    
    // 最終状態を表示
    println!("\n--- 最終状態 ---");
    scheduler.update(&mut world, 0.0);
    
    println!("🎉 ECSシステムのテストが完了しました！");
    println!("{}", "=".repeat(50));
}