#[actix_web::main]
async fn main() -> std::io::Result<()> {
    actix_web::HttpServer::new(move || {
        actix_web::App::new().route("/", actix_web::web::get().to(hello_world))
    })
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}

async fn hello_world() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().body("Hello, world!")
}
