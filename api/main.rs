use angren_quiz_core::{
    create_session_token, get_collection, get_public_questions, verify_session_token,
    ParticipantResult, SessionPayload, QUESTIONS,
};
use chrono::Utc;
use futures::stream::TryStreamExt;
use mongodb::{bson::doc, options::FindOptions};
use serde::Deserialize;
use serde_json::json;
use std::env;
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};

#[derive(Deserialize)]
struct StartReq {
    full_name: String,
    phone: String,
    school: String,
}

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
    let path = req.uri().path();

    match path {
        "/api/questions" => {
            let questions = get_public_questions();
            json_response(StatusCode::OK, json!({ "questions": questions }))
        }

        "/api/start" => {
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
            json_response(
                StatusCode::OK,
                json!({
                    "session_token": session_token,
                    "started_at_ms": now_ms
                }),
            )
        }

        "/api/submit" => {
            let body_bytes = req.body().to_vec();
            let sub: SubmitReq = serde_json::from_slice(&body_bytes)?;

            let session = match verify_session_token(&sub.session_token) {
                Some(s) => s,
                None => {
                    return json_response(
                        StatusCode::FORBIDDEN,
                        json!({ "error": "Yaroqsiz sessiya" }),
                    )
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

            let score = correct_count * 2; // 30 вопросов = 60 баллов

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

            json_response(
                StatusCode::OK,
                json!({
                    "score": score,
                    "correct_count": correct_count,
                    "time_ms": elapsed_ms
                }),
            )
        }

        "/api/leaderboard" => {
            let query = req.uri().query().unwrap_or("");
            let admin_key =
                env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "angren2026".to_string());
            let is_admin = query.contains(&format!("admin_key={}", admin_key));

            let coll = get_collection().await?;
            let opts = FindOptions::builder()
                .sort(doc! { "score": -1, "time_ms": 1 })
                .limit(500)
                .build();

            let cursor = coll.find(doc! {}).with_options(opts).await?;
            let list: Vec<_> = cursor.try_collect().await?;

            let output: Vec<_> = list
                .into_iter()
                .enumerate()
                .map(|(i, mut item)| {
                    if !is_admin {
                        let l = item.phone.len();
                        if l > 4 {
                            item.phone = format!("+998 ** *** {}", &item.phone[l - 4..]);
                        }
                    }
                    json!({
                        "rank": i + 1,
                        "full_name": item.full_name,
                        "phone": item.phone,
                        "school": item.school,
                        "score": item.score,
                        "correct_count": item.correct_count,
                        "time_ms": item.time_ms,
                        "created_at": item.created_at
                    })
                })
                .collect();

            json_response(StatusCode::OK, json!({ "leaderboard": output }))
        }

        _ => json_response(StatusCode::NOT_FOUND, json!({ "error": "Not found" })),
    }
}

fn json_response(status: StatusCode, body: serde_json::Value) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::Text(body.to_string()))?)
}
