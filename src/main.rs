
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt  = tokio::runtime::Builder::new_multi_thread()
      .worker_threads(4)
      .thread_stack_size(8 * 1024 * 1024)
      .build()?;

    rt.block_on(async {
      if let Err(e) = main_async().await {
        eprintln!("[ main_async ] {:?}", e);
      }
    });

    Ok(())
}

async fn main_async() -> Result<(), Box<dyn std::error::Error>> {

  println!("Hello async runtime!");

  Ok(())
}

