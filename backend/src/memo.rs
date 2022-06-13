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

    async fn insert(executor: impl sqlx::PgExecutor<'_>, memo: &Memo) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
        INSERT INTO memos
            (id, timestamp, done, text)
        VALUES($1, $2, $3, $4)
            "#,
        )
        .bind(&memo.id)
        .bind(&memo.timestamp)
        .bind(&memo.done)
        .bind(&memo.text)
        .execute(executor)
        .await
        .map(|_| ())
    }

    async fn update(
        executor: impl sqlx::PgExecutor<'_>,
        id: &uuid::Uuid,
        done: &bool,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query(
            r#"
        UPDATE memos
        SET done = $2
        WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(done)
        .execute(executor)
        .await
        .map(|result| result.rows_affected() > 0)
    }

    async fn delete(
        executor: impl sqlx::PgExecutor<'_>,
        id: &uuid::Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query(
            r#"
        DELETE FROM memos
        WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(executor)
        .await
        .map(|result| result.rows_affected() > 0)
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

#[derive(serde::Deserialize)]
pub(crate) struct NewMemoPayload {
    text: String,
}

pub(crate) async fn create(
    app_state: actix_web::web::Data<crate::AppState>,
    payload: actix_web::web::Json<NewMemoPayload>,
) -> actix_web::HttpResponse {
    let id = uuid::Uuid::new_v4();
    let memo = Memo {
        id,
        timestamp: chrono::Utc::now(),
        done: false,
        text: payload.into_inner().text,
    };
    match Memo::insert(&app_state.pool, &memo).await {
        Ok(_) => actix_web::HttpResponse::Ok().json(memo),
        Err(_) => actix_web::HttpResponse::InternalServerError().finish(),
    }
}

#[derive(serde::Deserialize)]
pub(crate) struct UpdateMemoPayload {
    id: uuid::Uuid,
    done: bool,
}

pub(crate) async fn resolve(
    app_state: actix_web::web::Data<crate::AppState>,
    payload: actix_web::web::Json<UpdateMemoPayload>,
) -> actix_web::HttpResponse {
    let payload = payload.into_inner();
    match Memo::update(&app_state.pool, &payload.id, &payload.done).await {
        Ok(deleted) => {
            if deleted {
                actix_web::HttpResponse::Ok().finish()
            } else {
                actix_web::HttpResponse::NotFound().finish()
            }
        }
        Err(_) => actix_web::HttpResponse::InternalServerError().finish(),
    }
}

#[derive(serde::Deserialize)]
pub(crate) struct DeleteMemoPayload {
    id: uuid::Uuid,
}

pub(crate) async fn delete(
    app_state: actix_web::web::Data<crate::AppState>,
    payload: actix_web::web::Json<DeleteMemoPayload>,
) -> actix_web::HttpResponse {
    let payload = payload.into_inner();
    match Memo::delete(&app_state.pool, &payload.id).await {
        Ok(deleted) => {
            if deleted {
                actix_web::HttpResponse::Ok().finish()
            } else {
                actix_web::HttpResponse::NotFound().finish()
            }
        }
        Err(_) => actix_web::HttpResponse::InternalServerError().finish(),
    }
}
