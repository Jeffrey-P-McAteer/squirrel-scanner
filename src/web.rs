
#[allow(unreachable_code)]
pub async fn run_webserver_forever() -> Result<(), Box<dyn std::error::Error>> {

  let mut num_webserver_restarts = 0;

  loop {
    if crate::PLEASE_EXIT_FLAG.load(std::sync::atomic::Ordering::Relaxed) {
      break;
    }
    num_webserver_restarts += 1;
    let mut caught_sigterm = false;
    if let Err(e) = run_webserver_once().await {
      eprintln!("[ run_webserver_once ] {:?}", e);
      let e_s = format!("{:?}", e);
      if e_s.contains("Interrupted") && e_s.contains("system") && e_s.contains("call") {
        caught_sigterm = true; // Cannot use .await points as long as {e} is in scope b/c not Send
      }
    }
    if caught_sigterm || num_webserver_restarts > 8 {
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
        .service(index)
        .service(style)
        .service(frame)
        .service(fast_frame)
        .service(shutdown)
  })
  .bind(("::", port))
  .expect("cannot bind to port")
  .run()
  .await?;

  Ok(())
}


#[actix_web::get("/")]
async fn index() -> actix_web::HttpResponse {
  actix_web::HttpResponse::Ok()
      .content_type(actix_web::http::header::ContentType(mime::TEXT_HTML_UTF_8))
      .body(
        &include_bytes!("web_index.html")[..]
      )
}

#[actix_web::get("/style.css")]
async fn style() -> actix_web::HttpResponse {
  actix_web::HttpResponse::Ok()
      .content_type(actix_web::http::header::ContentType(mime::TEXT_CSS_UTF_8))
      .body(
        &include_bytes!("web_style.css")[..]
      )
}


#[actix_web::get("/frame")]
async fn frame() -> actix_web::HttpResponse {
  if let Ok(encoded_img_bytes) = crate::camera::CAMERA_LAST_FRAME_JPEG.read() {
    let encoded_img_bytes = (*encoded_img_bytes).clone();
    actix_web::HttpResponse::Ok()
      .content_type(actix_web::http::header::ContentType(mime::IMAGE_JPEG))
      .insert_header(("Refresh", "2")) // Hint to browsers to refresh page after 2 seconds
      .body(encoded_img_bytes)
  }
  else {
    actix_web::HttpResponse::InternalServerError()
      .into()
  }
}


#[actix_web::get("/fast-frame")]
async fn fast_frame() -> actix_web::HttpResponse {
  if let Ok(encoded_img_bytes) = crate::camera::CAMERA_LAST_FRAME_JPEG.read() {
    let encoded_img_bytes = (*encoded_img_bytes).clone();
    actix_web::HttpResponse::Ok()
      .content_type(actix_web::http::header::ContentType(mime::IMAGE_JPEG))
      .insert_header(("Refresh", "1")) // Hint to browsers to refresh page after 1 second
      .body(encoded_img_bytes)
  }
  else {
    actix_web::HttpResponse::InternalServerError()
      .into()
  }
}


#[actix_web::get("/shutdown")]
async fn shutdown() -> impl actix_web::Responder {
  crate::utils::do_nice_shutdown().await;

  "Shutting Down..."
}



