//presentation/controller/fortune_controller.rs
// ランダム癒し系おみくじAPI
use axum::Json;
use rand::seq::SliceRandom;
use serde::Serialize;

#[derive(Serialize)]
pub struct FortuneResponse {
    pub message: String,
}

/// GET /api/fortune
pub async fn get_fortune() -> Json<FortuneResponse> {
    let fortunes = [
        "今日はきっと良いことがあるでしょう！",
        "リラックスして深呼吸をしましょう。",
        "小さな幸せを見つけてみてください。",
        "新しいことに挑戦するのに最適な日です。",
        "無理せず自分のペースで進みましょう。",
        "笑顔を忘れずに！",
        "周りの人に感謝の気持ちを伝えてみましょう。",
        "美味しいものを食べて元気をチャージ！",
        "今日はゆっくり休むのも大切です。",
        "あなたの努力は必ず報われます。",
    ];
    let mut rng = rand::thread_rng();
    let message = fortunes.choose(&mut rng).unwrap().to_string();
    Json(FortuneResponse { message })
}
