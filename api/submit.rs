use angren_bayroq::{get_collection, verify_session_token, ParticipantResult, QUESTIONS};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};

#[derive(Deserialize)]
struct SubmitReq {
    session_token: String,
    answers: Vec<usize>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    let body_bytes = req.body().to_vec();
    let sub: SubmitReq = serde_json::from_slice(&body_bytes)?;

    let session = match verify_session_token(&sub.session_token) {
        Some(s) => s,
        None => {
            return Ok(Response::builder()
                .status(StatusCode::FORBIDDEN)
                .header("Content-Type", "application/json")
                .body(Body::Text(json!({"error": "Yaroqsiz sessiya"}).to_string()))?)
        }
    };

    let now_ms = Utc::now().timestamp_millis();
    let elapsed_ms = (now_ms - session.started_at_ms).max(3000);

    let mut correct_count = 0;
    for (idx, q) in QUESTIONS.iter().enumerate() {
        if let Some(&ans) = sub.answers.get(idx) {
            if ans == q.correct_index {
                correct_count += 1;
            }
        }
    }

    let score = correct_count * 2; // 30 ta savol * 2 ball = maksimal 60 ball

    let record = ParticipantResult {
        id: None,
        full_name: session.full_name,
        phone: session.phone,
        school: session.school,
        score,
        correct_count,
        time_ms: elapsed_ms,
        created_at: Utc::now().to_rfc3339(),
    };

    let coll = get_collection().await?;
    coll.insert_one(&record).await?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::Text(
            json!({
                "score": score,
                "correct_count": correct_count,
                "time_ms": elapsed_ms
            })
            .to_string(),
        ))?)
}
