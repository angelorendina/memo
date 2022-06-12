#[derive(serde::Serialize)]
struct Memo {
    id: uuid::Uuid,
    timestamp: chrono::DateTime<chrono::Utc>,
    done: bool,
    text: String,
}

impl Memo {
    async fn index(executor: impl sqlx::PgExecutor<'_>) -> Result<Vec<Memo>, sqlx::Error> {
        sqlx::query_as!(
            Memo,
            r#"
        SELECT
            id, timestamp, done, text
        FROM memos
        ORDER BY timestamp
            "#,
        )
        .fetch_all(executor)
        .await
    }
}

pub(crate) async fn index(
    app_state: actix_web::web::Data<crate::AppState>,
) -> actix_web::HttpResponse {
    match Memo::index(&app_state.pool).await {
        Ok(memos) => actix_web::HttpResponse::Ok().json(memos),
        Err(_) => actix_web::HttpResponse::InternalServerError().finish(),
    }
}
