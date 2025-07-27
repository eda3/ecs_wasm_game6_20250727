// =============================================================================
// ECS WASM ソリティアゲーム - メインライブラリ
// =============================================================================
// このファイルはWebAssemblyとして公開される関数を定義し、
// ECSアーキテクチャを使ったマルチプレイソリティアゲームの
// エントリーポイントとして機能します。
//
// 設計思想：
// - Entity-Component-System (ECS) パターンを採用
// - 関数型プログラミングの原則に従い、状態の変更を最小化
// - 不変性を重視し、状態変更は明示的に行う
// - エラーハンドリングはResult型を使用してコンパイル時に保証
// =============================================================================

// WebAssembly機能が有効な場合のみインポート
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

// JavaScriptのコンソールログ出力用（WebAssembly機能有効時のみ）
#[cfg(feature = "wasm")]
#[wasm_bindgen]
extern "C" {
    // JavaScriptのconsole.logを使用してデバッグ出力を行う
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// マクロを定義：WebAssembly機能の有無に応じて出力先を切り替え
#[cfg(feature = "wasm")]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[cfg(not(feature = "wasm"))]
macro_rules! console_log {
    ($($t:tt)*) => (println!($($t)*))
}

// WebAssembly初期化時に実行される関数（WebAssembly機能有効時のみ）
// パニック時のエラー情報をブラウザのコンソールに出力するよう設定
#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn main() {
    // パニック時のスタックトレースをコンソールに出力
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    // 初期化完了をログ出力
    console_log!("🎮 ECS WASM ソリティアゲーム初期化完了！");
}

// =============================================================================
// パブリックAPI：JavaScriptから呼び出し可能な関数群
// =============================================================================

// ゲームの初期化（WebAssembly機能有効時のみ）
// 戻り値：初期化が成功したかどうかを示すブール値
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn initialize_game() -> bool {
    console_log!("🚀 ゲーム初期化開始...");
    
    // TODO: ECSワールドの初期化処理をここに追加
    // TODO: 初期コンポーネントとシステムの登録
    
    console_log!("✅ ゲーム初期化完了！");
    true
}

// 新しいゲームセッションを開始（WebAssembly機能有効時のみ）
// 引数：player_name - プレイヤー名
// 戻り値：セッションIDを表す文字列
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn start_new_game(player_name: &str) -> String {
    console_log!("🎯 新しいゲーム開始: プレイヤー「{}」", player_name);
    
    // TODO: 新しいゲームセッションの作成処理
    // TODO: ECSエンティティの生成とコンポーネントの初期化
    
    // 一時的なセッションID（後でUUID生成に変更予定）
    #[cfg(feature = "wasm")]
    let session_id = format!("session_{}", js_sys::Date::now() as u64);
    #[cfg(not(feature = "wasm"))]
    let session_id = format!("session_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    
    console_log!("📝 セッションID生成: {}", session_id);
    session_id
}

// ゲーム状態の更新（WebAssembly機能有効時のみ）
// デルタタイム（前回の更新からの経過時間）を受け取り、
// ECSシステムを実行してゲーム状態を進行させる
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn update_game(delta_time: f64) {
    // TODO: ECSシステムの実行
    // TODO: 各システムに delta_time を渡して状態更新
    
    // デバッグ用（本番では削除予定）
    if delta_time > 16.0 { // 60FPS以下の場合のみログ出力
        console_log!("⚠️  フレームレート低下検出: {}ms", delta_time);
    }
}

// WebSocket接続の状態を取得（WebAssembly機能有効時のみ）
// 戻り値：接続状態を表す文字列（"connected", "disconnected", "connecting"）
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn get_connection_status() -> String {
    // TODO: WebSocket接続状態の実装
    "disconnected".to_string()
}

// =============================================================================
// Windowsソリティア専用のWebAssembly API
// =============================================================================

// ソリティアゲームの状態を取得（WebAssembly機能有効時のみ）
// 戻り値：ゲーム状態をJSON文字列で返す
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn get_solitaire_state() -> String {
    use crate::solitaire::SolitaireManager;
    
    console_log!("📊 ソリティア状態取得リクエスト");
    
    // TODO: 実際のゲーム状態を取得
    // 現在はテスト用の状態を返す
    let test_state = serde_json::json!({
        "tableau": [
            [{"suit": "♠", "rank": "K", "face_up": true}],
            [{"suit": "♥", "rank": "Q", "face_up": false}, {"suit": "♣", "rank": "J", "face_up": true}],
            [{"suit": "♦", "rank": "10", "face_up": false}, {"suit": "♠", "rank": "9", "face_up": false}, {"suit": "♥", "rank": "8", "face_up": true}],
            // ... 他の列
        ],
        "foundation": [[], [], [], []], // 4つのファウンデーション
        "deck_count": 24,
        "waste": [{"suit": "♣", "rank": "7", "face_up": true}],
        "moves": 0,
        "score": 0,
        "time_elapsed": 0
    });
    
    test_state.to_string()
}

// カードを移動する（WebAssembly機能有効時のみ）
// 引数：from_location, to_location - 移動元と移動先の位置情報（JSON文字列）
// 戻り値：移動が成功したかどうかを示すブール値
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn move_card(from_location: &str, to_location: &str) -> bool {
    console_log!("🎯 カード移動: {} -> {}", from_location, to_location);
    
    // TODO: 実際の移動処理を実装
    // 現在はテスト用に常にtrueを返す
    
    // JSONパースのテスト
    match (serde_json::from_str::<serde_json::Value>(from_location), 
           serde_json::from_str::<serde_json::Value>(to_location)) {
        (Ok(from), Ok(to)) => {
            console_log!("✅ 移動先パース成功: {:?} -> {:?}", from, to);
            true
        },
        _ => {
            console_log!("❌ 移動先パース失敗");
            false
        }
    }
}

// デッキからカードを引く（WebAssembly機能有効時のみ）
// 戻り値：引いたカードの情報をJSON文字列で返す（引けない場合は空文字列）
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn draw_card_from_deck() -> String {
    console_log!("🎴 デッキからカードを引く");
    
    // TODO: 実際のデッキ処理を実装
    // 現在はテスト用のランダムカードを返す
    
    use js_sys::Math;
    let suits = ["♠", "♥", "♦", "♣"];
    let ranks = ["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];
    
    let suit_index = (Math::random() * 4.0) as usize;
    let rank_index = (Math::random() * 13.0) as usize;
    
    let card = serde_json::json!({
        "suit": suits[suit_index],
        "rank": ranks[rank_index],
        "face_up": true
    });
    
    console_log!("🎴 引いたカード: {}", card.to_string());
    card.to_string()
}

// ゲームのリセット（WebAssembly機能有効時のみ）
// 戻り値：リセットが成功したかどうかを示すブール値
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn reset_solitaire_game() -> bool {
    console_log!("🔄 ソリティアゲームをリセット");
    
    // TODO: 実際のリセット処理を実装
    // - カードの再配布
    // - スコアのリセット
    // - タイマーのリセット
    
    console_log!("✅ ゲームリセット完了");
    true
}

// 自動配置を試行（WebAssembly機能有効時のみ）
// 引数：card_info - カード情報（JSON文字列）
// 戻り値：自動配置が成功したかどうかを示すブール値
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn try_auto_place(card_info: &str) -> bool {
    console_log!("🚀 自動配置試行: {}", card_info);
    
    // TODO: 実際の自動配置ロジックを実装
    // - ファウンデーションへの配置チェック
    // - タブローへの配置チェック
    
    // テスト用：50%の確率で成功
    let success = js_sys::Math::random() > 0.5;
    
    if success {
        console_log!("✨ 自動配置成功");
    } else {
        console_log!("⚠️ 自動配置失敗");
    }
    
    success
}

// 勝利条件をチェック（WebAssembly機能有効時のみ）
// 戻り値：ゲームが完了したかどうかを示すブール値
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn check_victory() -> bool {
    console_log!("🏆 勝利条件チェック");
    
    // TODO: 実際の勝利条件チェックを実装
    // - 全てのカードがファウンデーションに配置されているかチェック
    
    false // 現在は常にfalse
}

// ヒントを取得（WebAssembly機能有効時のみ）
// 戻り値：ヒント情報をJSON文字列で返す
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn get_hint() -> String {
    console_log!("💡 ヒント取得");
    
    // TODO: 実際のヒント生成ロジックを実装
    
    let hint = serde_json::json!({
        "type": "move",
        "message": "♥のKをファウンデーションに移動できます",
        "from": {"type": "tableau", "column": 0},
        "to": {"type": "foundation", "suit": "♥"}
    });
    
    console_log!("💡 ヒント生成: {}", hint.to_string());
    hint.to_string()
}

// =============================================================================
// WebAssemblyメモリの最適化
// =============================================================================

// 軽量なアロケータを使用（WebAssembly向け最適化）
#[cfg(all(feature = "wasm", feature = "wee_alloc"))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// =============================================================================
// モジュール宣言：他のファイルで定義される機能
// =============================================================================

// ECS関連のモジュール
mod ecs;       // ECSコンポーネント実装完了により有効化
mod game;      // ゲーム状態管理システム実装完了により有効化
mod network;   // WebSocket通信レイヤ実装完了により有効化
mod solitaire; // ソリティアゲームロジック実装完了により有効化