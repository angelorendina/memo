#[derive(serde::Deserialize, serde::Serialize)]
pub struct Memo {
    pub id: uuid::Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub done: bool,
    pub text: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct NewMemoPayload {
    pub text: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct UpdateMemoPayload {
    pub id: uuid::Uuid,
    pub done: bool,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DeleteMemoPayload {
    pub id: uuid::Uuid,
}
