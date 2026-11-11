use angren_bayroq::get_collection;
use futures::stream::TryStreamExt;
use mongodb::{bson::doc, options::FindOptions};
use serde_json::json;
use std::env;
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    let query = req.uri().query().unwrap_or("");
    let admin_key = env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "angren2026".to_string());
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
                // Oddiy foydalanuvchilarga telefon raqamning faqat oxirgi qismi ko'rinadi
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

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::Text(json!({ "leaderboard": output }).to_string()))?)
}
