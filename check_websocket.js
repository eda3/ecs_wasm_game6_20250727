// WebSocket接続確認スクリプト (Node.js用)
// 使用方法: node check_websocket.js

const WebSocket = require('ws');

console.log('🔌 WebSocket接続テスト開始...');
console.log('サーバー: ws://162.43.8.148:8101');

const ws = new WebSocket('ws://162.43.8.148:8101');

let connected = false;

// 接続成功
ws.on('open', function open() {
    console.log('✅ WebSocket接続成功！');
    connected = true;
    
    // テストメッセージを送信
    const testMessage = {
        type: 'PlayerJoin',
        player_id: 'test_node_client',
        player_name: 'NodeTestClient'
    };
    
    console.log('📤 テストメッセージ送信:', JSON.stringify(testMessage));
    ws.send(JSON.stringify(testMessage));
    
    // 3秒後に切断
    setTimeout(() => {
        console.log('🔌 テスト完了、切断します');
        ws.close();
    }, 3000);
});

// メッセージ受信
ws.on('message', function message(data) {
    console.log('📥 受信:', data.toString());
});

// 接続終了
ws.on('close', function close() {
    console.log('❌ WebSocket接続終了');
    process.exit(connected ? 0 : 1);
});

// エラー
ws.on('error', function error(err) {
    console.log('❌ WebSocket接続エラー:', err.message);
    console.log('💡 サーバーが起動していることを確認してください');
    process.exit(1);
});

// タイムアウト設定
setTimeout(() => {
    if (!connected) {
        console.log('⏰ 接続タイムアウト（10秒）');
        console.log('💡 サーバーが起動していることを確認してください');
        ws.close();
        process.exit(1);
    }
}, 10000);