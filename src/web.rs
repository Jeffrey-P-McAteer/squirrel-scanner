
#[allow(unreachable_code)]
pub async fn run_webserver_forever() -> Result<(), Box<dyn std::error::Error>> {

  loop {
    if let Err(e) = run_webserver_once().await {
      eprintln!("[ run_webserver_once ] {:?}", e)
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
  }

  Ok(())
}


pub async fn run_webserver_once() -> Result<(), Box<dyn std::error::Error>> {

  actix_web::HttpServer::new(|| {
      actix_web::App::new()
        .service(greet)
  })
  .bind(("::", 8080))
  .expect("cannot bind to port")
  .run()
  .await?;

  Ok(())
}


#[actix_web::get("/hello/{name}")]
async fn greet(name: actix_web::web::Path<String>) -> impl actix_web::Responder {
    format!("Hello {}!", name)
}



