use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

// ================================================================
// 設定項目
// ================================================================
const API_SECRET_KEY: &str = "0228";
const WATER_PUMP_PIN: u8 = 17;
const WATER_DURATION_SECS: u64 = 5;

// ================================================================
// GPIO制御の抽象化
// ================================================================
#[cfg(feature = "gpio")]
use rppal::gpio::Gpio;

enum GpioController {
    #[cfg(feature = "gpio")]
    Real(rppal::gpio::OutputPin),
    Dummy,
}

impl GpioController {
    fn new(pin: u8) -> Self {
        #[cfg(feature = "gpio")]
        {
            match Gpio::new() {
                Ok(gpio) => match gpio.get(pin) {
                    Ok(pin) => {
                        let output = pin.into_output_low();
                        info!("✅ GPIO初期化成功: ピン {} を出力に設定", WATER_PUMP_PIN);
                        return Self::Real(output);
                    }
                    Err(e) => {
                        warn!("⚠️ GPIOピン取得失敗: {}. ダミーモードで起動", e);
                    }
                },
                Err(e) => {
                    warn!("⚠️ GPIO初期化失敗: {}. ダミーモードで起動", e);
                }
            }
        }
        
        info!("⚠️ ダミーモードで起動します");
        Self::Dummy
    }

    async fn run_motor(&mut self, duration: Duration) -> Result<String, String> {
        match self {
            #[cfg(feature = "gpio")]
            Self::Real(pin) => {
                info!("🚀 モーターON");
                pin.set_high();
                tokio::time::sleep(duration).await;
                pin.set_low();
                info!("🛑 モーターOFF");
                Ok(format!("実機実行: {}秒間モーターを制御しました", duration.as_secs()))
            }
            Self::Dummy => {
                info!("--- [DUMMY MODE] モーター動作シミュレーション: {}秒 ---", duration.as_secs());
                tokio::time::sleep(duration).await;
                info!("--- [DUMMY MODE] 動作完了 ---");
                Ok(format!("ダミー実行: {}秒間モーターを制御しました", duration.as_secs()))
            }
        }
    }

    fn is_dummy(&self) -> bool {
        matches!(self, Self::Dummy)
    }
}

// ================================================================
// アプリケーション状態
// ================================================================
struct AppState {
    gpio: tokio::sync::Mutex<GpioController>,
}

// ================================================================
// API型定義
// ================================================================
#[derive(Serialize)]
struct StatusResponse {
    status: String,
    message: String,
    server_mode: String,
    controlled_pin: u8,
}

#[derive(Deserialize)]
struct WaterRequest {
    action: String,
}

#[derive(Serialize)]
struct WaterResponse {
    status: String,
    message: String,
    gpio_result: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

// ================================================================
// ミドルウェア: APIキー検証
// ================================================================
async fn validate_api_key(headers: &HeaderMap) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    match headers.get("X-API-KEY") {
        Some(key) if key == API_SECRET_KEY => Ok(()),
        _ => Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized: Invalid API Key".to_string(),
            }),
        )),
    }
}

// ================================================================
// APIハンドラー
// ================================================================
async fn status_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<StatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    validate_api_key(&headers).await?;

    let gpio = state.gpio.lock().await;
    let mode = if gpio.is_dummy() {
        "DUMMY MODE"
    } else {
        "LIVE RPi.GPIO MODE"
    };

    Ok(Json(StatusResponse {
        status: "Ready".to_string(),
        message: "Server is ready".to_string(),
        server_mode: mode.to_string(),
        controlled_pin: WATER_PUMP_PIN,
    }))
}

async fn water_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<WaterRequest>,
) -> Result<Json<WaterResponse>, (StatusCode, Json<ErrorResponse>)> {
    validate_api_key(&headers).await?;

    if payload.action != "start" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid request body. Expected {'action': 'start'}".to_string(),
            }),
        ));
    }

    info!("[リクエスト受信] 水やり ({}秒)", WATER_DURATION_SECS);

    let mut gpio = state.gpio.lock().await;
    let result = gpio
        .run_motor(Duration::from_secs(WATER_DURATION_SECS))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("GPIO操作エラー: {}", e),
                }),
            )
        })?;

    Ok(Json(WaterResponse {
        status: "success".to_string(),
        message: format!("水やり ({}秒) が完了しました", WATER_DURATION_SECS),
        gpio_result: result,
    }))
}

// ================================================================
// メイン関数
// ================================================================
#[tokio::main]
async fn main() {
    // ログ初期化
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    info!("--- Rust Axum APIサーバー起動 ---");
    info!("制御ピン (BCM): {}", WATER_PUMP_PIN);

    // GPIO初期化
    let gpio = GpioController::new(WATER_PUMP_PIN);
    let app_state = Arc::new(AppState {
        gpio: tokio::sync::Mutex::new(gpio),
    });

    // CORSの設定
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // ルーター構築
    let app = Router::new()
        .route("/status", get(status_handler))
        .route("/water", post(water_handler))
        .layer(cors)
        .with_state(app_state);

    // サーバー起動
    let addr = SocketAddr::from(([0, 0, 0, 0], 5000));
    info!("🚀 サーバー起動: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
