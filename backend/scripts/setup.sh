#!/bin/bash
set -e

echo "🍓 Raspberry Pi 水やりバックエンド セットアップスクリプト"
echo "================================================"

# カレントディレクトリを確認
PROJECT_DIR="$HOME/watering-backend"
if [ ! -f "$PROJECT_DIR/Cargo.toml" ]; then
    echo "❌ エラー: Cargo.tomlが見つかりません"
    echo "   このスクリプトは $PROJECT_DIR で実行してください"
    exit 1
fi

cd "$PROJECT_DIR"

echo ""
echo "📦 Step 1: 必要なパッケージをインストール..."
sudo apt-get update
sudo apt-get install -y build-essential

echo ""
echo "🔧 Step 2: GPIOアクセス権限を設定..."
if ! groups $USER | grep -q gpio; then
    sudo usermod -a -G gpio $USER
    echo "✅ ユーザー $USER を gpio グループに追加しました"
    echo "⚠️  反映には再ログインが必要です"
else
    echo "✅ すでにgpioグループに所属しています"
fi

echo ""
echo "🦀 Step 3: Rustプロジェクトをビルド (GPIO機能有効)..."
cargo build --release --features gpio

if [ ! -f "$PROJECT_DIR/target/release/watering-backend" ]; then
    echo "❌ ビルド失敗: バイナリが生成されませんでした"
    exit 1
fi

echo "✅ ビルド完了"

echo ""
echo "⚙️  Step 4: systemdサービスを登録..."

# サービスファイルを作成
sudo tee /etc/systemd/system/watering-backend.service > /dev/null <<EOF
[Unit]
Description=Watering System Backend (Rust)
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$PROJECT_DIR
ExecStart=$PROJECT_DIR/target/release/watering-backend
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

echo "✅ サービスファイルを作成しました"

echo ""
echo "🚀 Step 5: サービスを有効化して起動..."
sudo systemctl daemon-reload
sudo systemctl enable watering-backend
sudo systemctl start watering-backend

echo ""
echo "⏳ サービスの起動を待っています..."
sleep 3

echo ""
echo "📊 サービスの状態を確認:"
sudo systemctl status watering-backend --no-pager || true

echo ""
echo "================================================"
echo "✅ セットアップ完了!"
echo ""
echo "📝 次のステップ:"
echo "   1. 動作確認: curl -H 'X-API-KEY: 0228' http://localhost:5000/status"
echo "   2. ログ確認: sudo journalctl -u watering-backend -f"
echo ""
echo "🔄 コード更新時は ./scripts/update.sh を実行してください"
echo "================================================"
