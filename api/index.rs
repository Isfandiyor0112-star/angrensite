use angren_quiz_core::{
    create_session_token, get_collection, get_public_questions, verify_session_token,
    ParticipantResult, SessionPayload, QUESTIONS,
};
use chrono::Utc;
use futures::stream::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId},
    options::FindOptions,
};
use serde::Deserialize;
use serde_json::json;
use std::{env, str::FromStr};
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

#[derive(Deserialize)]
struct DeleteReq {
    admin_key: String,
    id: String,
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

            let coll = get_collection().await?;

            // Защита от повторной отправки одного и того же теста
            let recent_cutoff = (Utc::now().timestamp_millis() - 15000).to_string();
            if let Ok(Some(existing)) = coll
                .find_one(doc! {
                    "full_name": &session.full_name,
                    "phone": &session.phone,
                    "school": &session.school,
                    "created_at": { "$gte": recent_cutoff }
                })
                .await
            {
                return json_response(
                    StatusCode::OK,
                    json!({
                        "score": existing.score,
                        "correct_count": existing.correct_count,
                        "time_ms": existing.time_ms
                    }),
                );
            }

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

            let score = correct_count * 2; // 30 вопросов * 2 балла = 60 баллов

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
            let expected_key =
                env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "angren2026".to_string());

            let mut provided_key = None;
            for pair in query.split('&') {
                if let Some(val) = pair.strip_prefix("admin_key=") {
                    provided_key = Some(val);
                }
            }

            let is_admin = match provided_key {
                Some(k) => {
                    if k != expected_key {
                        return json_response(
                            StatusCode::UNAUTHORIZED,
                            json!({ "error": "Noto'g'ri parol" }),
                        );
                    }
                    true
                }
                None => false,
            };

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
                    let id_str = item.id.map(|oid| oid.to_hex()).unwrap_or_default();
                    if !is_admin {
                        let l = item.phone.len();
                        if l > 4 {
                            item.phone = format!("+998 ** *** {}", &item.phone[l - 4..]);
                        }
                    }
                    json!({
                        "id": id_str,
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

        "/api/delete" => {
            let body_bytes = req.body().to_vec();
            let del: DeleteReq = serde_json::from_slice(&body_bytes)?;
            let expected_key =
                env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "angren2026".to_string());

            if del.admin_key != expected_key {
                return json_response(
                    StatusCode::UNAUTHORIZED,
                    json!({ "error": "Noto'g'ri parol" }),
                );
            }

            let coll = get_collection().await?;
            if let Ok(oid) = ObjectId::from_str(&del.id) {
                coll.delete_one(doc! { "_id": oid }).await?;
            }

            json_response(StatusCode::OK, json!({ "deleted": true }))
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
