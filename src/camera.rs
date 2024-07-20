

#[allow(unreachable_code)]
pub async fn camera_loop() -> Result<(), Box<dyn std::error::Error>> {
  use v4l::video::Capture;

  let mut video_device_path = "/dev/video2".to_string();
  if let Ok(val) = std::env::var("VDEV") {
    video_device_path = val.to_string();
  }
  println!("Using device {}", &video_device_path);

  let mut dev = v4l::Device::with_path(&video_device_path)?;

  // Let's say we want to explicitly request another format
  let mut fmt = dev.format()?;
  fmt.width = 1920;
  fmt.height = 1080;
  fmt.fourcc = v4l::FourCC::new(b"Y416"); // https://stackoverflow.com/a/47736923

  let assigned_fmt = dev.set_format(&fmt)?;

  // The actual format chosen by the device driver may differ from what we requested
  println!("Camera Image Format in use:\n{}", assigned_fmt);

  if assigned_fmt.fourcc != fmt.fourcc {
    eprintln!("Did not get the fourcc we wanted! Wanted {}, got {}", fmt.fourcc, assigned_fmt.fourcc);
    return Ok(());
  }

  let cam_fmt_h = fmt.height as usize;
  let cam_fmt_w = fmt.width as usize;
  let img_bpp = (fmt.size / (fmt.height * fmt.width)) as usize;
  println!("Camera img_bpp = {:?}", img_bpp);

  // To achieve the best possible performance, you may want to use a
  // UserBufferStream instance, but this is not supported on all devices,
  // so we stick to the mapped case for this example.
  // Please refer to the rustdoc docs for a more detailed explanation about
  // buffer transfers.

  // Create the stream, which will internally 'allocate' (as in map) the
  // number of requested buffers for us.
  let mut stream = v4l::io::mmap::Stream::with_buffers(&mut dev, v4l::buffer::Type::VideoCapture, 1)?;

  loop {

    // TODO
    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;


    // Exit if requested by another component
    if crate::PLEASE_EXIT_FLAG.load(std::sync::atomic::Ordering::Relaxed) {
      break;
    }
  }

  Ok(())
}



