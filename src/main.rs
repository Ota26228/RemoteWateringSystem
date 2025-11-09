use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use yew::prelude::*;

// ================================================================
// 設定項目 - ⚠️ ここをあなたのRaspberry PiのIPアドレスに変更してください
// ================================================================
const API_BASE_URL: &str = "/api";  // ← Raspberry PiのTailscale IP
const API_KEY: &str = "0228";

// ================================================================
// API型定義
// ================================================================
#[derive(Deserialize, Clone, PartialEq)]
struct StatusResponse {
    status: String,
    message: String,
    server_mode: String,
    controlled_pin: u8,
}

#[derive(Serialize)]
struct WaterRequest {
    action: String,
}

#[derive(Deserialize, Clone)]
struct WaterResponse {
    status: String,
    message: String,
    gpio_result: String,
}

// ================================================================
// アプリケーション状態
// ================================================================
#[derive(Clone, PartialEq)]
enum ConnectionType {
    Remote,
    None,
}

impl ConnectionType {
    fn as_str(&self) -> &str {
        match self {
            Self::Remote => "外部 (Tailscale)",
            Self::None => "未接続",
        }
    }
}

// ================================================================
// メインコンポーネント
// ================================================================
#[function_component(App)]
fn app() -> Html {
    let connection_type = use_state(|| ConnectionType::None);
    let status = use_state(|| "待機中".to_string());
    let is_loading = use_state(|| false);
    let error = use_state(|| None::<String>);

    // サーバー状態確認
    let check_server_status = {
        let connection_type = connection_type.clone();
        let status = status.clone();
        let is_loading = is_loading.clone();
        let error = error.clone();

        Callback::from(move |_: web_sys::MouseEvent| {
            let connection_type = connection_type.clone();
            let status = status.clone();
            let is_loading = is_loading.clone();
            let error = error.clone();

            wasm_bindgen_futures::spawn_local(async move {
                error.set(None);
                status.set("サーバーに接続中...".to_string());
                is_loading.set(true);
                connection_type.set(ConnectionType::None);

                match Request::get(&format!("{}/status", API_BASE_URL))
                    .header("X-API-KEY", API_KEY)
                    .send()
                    .await
                {
                    Ok(response) if response.ok() => {
                        match response.json::<StatusResponse>().await {
                            Ok(data) => {
                                status.set(format!("✅ 接続完了: {}", data.message));
                                connection_type.set(ConnectionType::Remote);
                            }
                            Err(e) => {
                                error.set(Some(format!("レスポンス解析エラー: {}", e)));
                                status.set("エラー".to_string());
                            }
                        }
                    }
                    Ok(response) => {
                        error.set(Some(format!(
                            "サーバーが応答しません (ステータス: {})",
                            response.status()
                        )));
                        status.set("エラー".to_string());
                    }
                    Err(e) => {
                        error.set(Some(format!("接続失敗: {}", e)));
                        status.set("エラー".to_string());
                    }
                }

                is_loading.set(false);
            });
        })
    };

    // 水やりリクエスト
    let handle_watering = {
        let status = status.clone();
        let is_loading = is_loading.clone();
        let error = error.clone();

        Callback::from(move |_: web_sys::MouseEvent| {
            let status = status.clone();
            let is_loading = is_loading.clone();
            let error = error.clone();

            wasm_bindgen_futures::spawn_local(async move {
                is_loading.set(true);
                error.set(None);
                status.set("水やり開始をリクエスト中...".to_string());

                let request_body = WaterRequest {
                    action: "start".to_string(),
                };

                match Request::post(&format!("{}/water", API_BASE_URL))
                    .header("X-API-KEY", API_KEY)
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                {
                    Ok(request) => match request.send().await {
                        Ok(response) if response.ok() => {
                            match response.json::<WaterResponse>().await {
                                Ok(result) => {
                                    status.set(format!("✅ 成功: {}", result.message));
                                }
                                Err(e) => {
                                    error.set(Some(format!("レスポンス解析エラー: {}", e)));
                                    status.set("エラー".to_string());
                                }
                            }
                        }
                        Ok(response) if response.status() == 401 => {
                            error.set(Some("認証失敗: APIキーが間違っています".to_string()));
                            status.set("認証失敗".to_string());
                        }
                        Ok(response) => {
                            error.set(Some(format!("サーバーエラー: {}", response.status())));
                            status.set("通信失敗".to_string());
                        }
                        Err(e) => {
                            error.set(Some(format!("通信失敗: {}", e)));
                            status.set("通信失敗".to_string());
                        }
                    },
                    Err(e) => {
                        error.set(Some(format!("リクエスト作成エラー: {}", e)));
                        status.set("エラー".to_string());
                    }
                }

                is_loading.set(false);
            });
        })
    };

    // 初回読み込み時にサーバー状態確認
    {
        let check_server_status = check_server_status.clone();
        use_effect_with((), move |_| {
            // 初回は MouseEvent なしで呼ぶため、ダミーイベントを作成
            let event = web_sys::MouseEvent::new("click").unwrap();
            check_server_status.emit(event);
            || ()
        });
    }

    // ステータスカラー
    let status_color = if *is_loading {
        "bg-yellow-500"
    } else if error.is_some() {
        "bg-red-500"
    } else if status.contains("成功") || status.contains("接続完了") {
        "bg-green-500"
    } else if *connection_type == ConnectionType::Remote {
        "bg-blue-500"
    } else {
        "bg-gray-400"
    };

    // ボタンの有効/無効状態
    let is_water_disabled = *is_loading || *connection_type == ConnectionType::None;

    html! {
        <div class="min-h-screen bg-gray-100 flex items-center justify-center p-4">
            <div class="w-full max-w-sm bg-white p-6 rounded-xl shadow-2xl">
                <h1 class="text-3xl font-bold text-gray-800 mb-2 flex items-center">
                    <svg class="mr-2 text-blue-500" width="30" height="30" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M12 2.69l5.66 5.66a8 8 0 1 1-11.31 0z"/>
                    </svg>
                    {"リモート水やりコントローラー"}
                </h1>
                <p class="text-sm text-gray-500 mb-6">
                    {"Tailscale経由でRaspberry Piに接続します"}
                </p>

                // ステータスカード
                <div class={format!("p-4 rounded-lg text-white mb-6 transition-colors duration-300 {}", status_color)}>
                    <div class="flex items-center justify-between">
                        <span class="font-semibold">{&*status}</span>
                        <div class="flex items-center">
                            <svg class="mr-1" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <circle cx="12" cy="12" r="10"/>
                                <line x1="2" y1="12" x2="22" y2="12"/>
                                <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
                            </svg>
                            <span class="text-sm">{connection_type.as_str()}</span>
                        </div>
                    </div>
                    <p class="text-xs mt-1 truncate opacity-80">
                        {format!("接続先: {}", API_BASE_URL)}
                    </p>
                </div>

                // エラー表示
                if let Some(err) = &*error {
                    <div class="bg-red-100 border-l-4 border-red-500 text-red-700 p-3 mb-6 rounded-md">
                        <p class="font-bold">{"エラー"}</p>
                        <p class="text-sm">{err}</p>
                    </div>
                }

                // コントロールボタン
                <div class="space-y-4">
                    <button
                        onclick={handle_watering}
                        disabled={is_water_disabled}
                        class={format!(
                            "w-full flex items-center justify-center py-3 rounded-xl text-lg font-semibold transition-all duration-200 {}",
                            if is_water_disabled {
                                "bg-blue-300 cursor-not-allowed"
                            } else {
                                "bg-blue-600 hover:bg-blue-700 active:bg-blue-800 text-white shadow-lg transform active:scale-98"
                            }
                        )}
                    >
                        <svg class="mr-2" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M12 2.69l5.66 5.66a8 8 0 1 1-11.31 0z"/>
                        </svg>
                        {if *is_loading { "処理中..." } else { "水やり開始 (モーターON)" }}
                    </button>

                    <button
                        onclick={check_server_status}
                        disabled={*is_loading}
                        class={format!(
                            "w-full flex items-center justify-center py-3 rounded-xl text-lg font-semibold border border-gray-300 transition-all duration-200 {}",
                            if *is_loading {
                                "bg-gray-200 cursor-not-allowed text-gray-500"
                            } else {
                                "bg-white hover:bg-gray-50 text-gray-700 transform active:scale-98 shadow"
                            }
                        )}
                    >
                        <svg class="mr-2" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <polyline points="23 4 23 10 17 10"/>
                            <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
                        </svg>
                        {"接続を再確認"}
                    </button>
                </div>

                <p class="text-xs text-gray-400 mt-6 text-center">
                    {"※ Raspberry Pi で watering-backend が起動している必要があります"}
                </p>
            </div>
        </div>
    }
}

// ================================================================
// エントリーポイント
// ================================================================
fn main() {
    yew::Renderer::<App>::new().render();
}
