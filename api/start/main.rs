use angren_bayroq::{create_session_token, SessionPayload};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};

#[derive(Deserialize)]
struct StartReq {
    full_name: String,
    phone: String,
    school: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    let body_bytes = req.body().to_vec();
    let data: StartReq = serde_json::from_slice(&body_bytes)?;

    let now_ms = Utc::now().timestamp_millis();
    let payload = SessionPayload {
        full_name: data.full_name.trim().to_string(),
        phone: data.phone.trim().to_string(),
        school: data.school.trim().to_string(),
        started_at_ms: now_ms,
    };

    let session_token = create_session_token(&payload);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::Text(
            json!({
                "session_token": session_token,
                "started_at_ms": now_ms
            })
            .to_string(),
        ))?)
}
