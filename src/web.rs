
#[allow(unreachable_code)]
pub async fn run_webserver_forever() -> Result<(), Box<dyn std::error::Error>> {

  loop {
    if crate::PLEASE_EXIT_FLAG.load(std::sync::atomic::Ordering::Relaxed) {
      break;
    }

    let mut caught_sigterm = false;
    if let Err(e) = run_webserver_once().await {
      eprintln!("[ run_webserver_once ] {:?}", e);
      let e_s = format!("{:?}", e);
      if e_s.contains("Interrupted") && e_s.contains("system") && e_s.contains("call") {
        caught_sigterm = true; // Cannot use .await points as long as {e} is in scope b/c not Send
      }
    }
    if caught_sigterm {
      // We see this on ctrl+c SIGTERM events, so play nice & decide to exit.
      crate::utils::do_nice_shutdown().await;
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
async fn frame() -> actix_web::HttpResponse {
  let png_bytes_len = crate::camera::LAST_FRAME_PNG_BYTES_WRITTEN.load(std::sync::atomic::Ordering::SeqCst);
  actix_web::HttpResponse::Ok()
    .content_type(actix_web::http::header::ContentType(mime::IMAGE_PNG))
    //.insert_header(("X-Hdr", "sample"))
    .body(&crate::camera::LAST_FRAME_PNG[..png_bytes_len])
}


#[actix_web::get("/shutdown")]
async fn shutdown() -> impl actix_web::Responder {
  crate::utils::do_nice_shutdown().await;

  "Shutting Down..."
}



