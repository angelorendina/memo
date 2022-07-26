mod memo;

struct AppState {
    pool: sqlx::Pool<sqlx::Postgres>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let database_url = std::env!("DATABASE_URL");
    let server_port = std::env!("BACKEND_PORT")
        .parse::<u16>()
        .expect("Port must be a u16");

    let app_state = actix_web::web::Data::new(AppState {
        pool: sqlx::pool::PoolOptions::new()
            .connect(&database_url)
            .await
            .expect("Could not connect to the DB"),
    });

    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(app_state.clone())
            .wrap(actix_web::middleware::Logger::default())
            .wrap(actix_cors::Cors::permissive())
            .route("/", actix_web::web::get().to(memo::index))
            .route("/", actix_web::web::post().to(memo::create))
            .route("/", actix_web::web::put().to(memo::resolve))
            .route("/", actix_web::web::delete().to(memo::delete))
    })
    .bind(("0.0.0.0", server_port))?
    .run()
    .await
}
