mod memo;

struct AppState {
    pool: sqlx::Pool<sqlx::Postgres>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = actix_web::web::Data::new(AppState {
        pool: sqlx::pool::PoolOptions::new()
            .connect("postgresql://user:password@postgres:5432/db")
            .await
            .expect("Could not connect to the DB"),
    });

    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(app_state.clone())
            .route("/", actix_web::web::get().to(memo::index))
            .route("/", actix_web::web::post().to(memo::create))
            .route("/", actix_web::web::put().to(memo::resolve))
            .route("/", actix_web::web::delete().to(memo::delete))
    })
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}
