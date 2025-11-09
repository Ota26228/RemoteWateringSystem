#!/bin/bash
set -e

echo "🔄 水やりバックエンド 更新スクリプト"
echo "================================================"

PROJECT_DIR="$HOME/watering-backend"
cd "$PROJECT_DIR"

echo ""
echo "🛑 Step 1: 既存のサービスを停止..."
sudo systemctl stop watering-backend

echo ""
echo "🦀 Step 2: 最新のコードをビルド..."
cargo build --release --features gpio

if [ ! -f "$PROJECT_DIR/target/release/watering-backend" ]; then
    echo "❌ ビルド失敗: バイナリが生成されませんでした"
    echo "🔙 サービスを再起動します..."
    sudo systemctl start watering-backend
    exit 1
fi

echo "✅ ビルド完了"

echo ""
echo "🚀 Step 3: サービスを再起動..."
sudo systemctl start watering-backend

echo ""
echo "⏳ サービスの起動を待っています..."
sleep 3

echo ""
echo "📊 サービスの状態:"
sudo systemctl status watering-backend --no-pager || true

echo ""
echo "📋 最新のログ (最後の10行):"
sudo journalctl -u watering-backend -n 10 --no-pager

echo ""
echo "================================================"
echo "✅ 更新完了!"
echo ""
echo "💡 リアルタイムでログを確認:"
echo "   sudo journalctl -u watering-backend -f"
echo "================================================"
