
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

  let mut port = 8080;
  if std::fs::metadata("/proc/self").as_ref().map(|m| std::os::unix::fs::MetadataExt::uid(m) ).unwrap_or(1) == 0 {
    port = 80;
  }

  println!("Running webserver on http://localhost:{}", port);

  actix_web::HttpServer::new(|| {
      actix_web::App::new()
        .service(frame)
        .service(shutdown)
  })
  .bind(("::", port))
  .expect("cannot bind to port")
  .run()
  .await?;

  Ok(())
}


#[actix_web::get("/frame")]
async fn frame() -> impl actix_web::Responder {
    format!("Hello !")
}


#[actix_web::get("/shutdown")]
async fn shutdown() -> impl actix_web::Responder {
  crate::PLEASE_EXIT_FLAG.store(true, std::sync::atomic::Ordering::SeqCst);

  tokio::task::spawn(async { // Allow /shutdown to serve a last response, then shutdown the webserver task.
    tokio::time::sleep(tokio::time::Duration::from_millis(350)).await;
    actix_web::rt::System::current().stop();
  });

  "Shutting Down..."
}



