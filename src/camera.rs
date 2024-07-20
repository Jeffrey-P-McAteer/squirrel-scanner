

#[allow(unreachable_code)]
pub async fn camera_loop() -> Result<(), Box<dyn std::error::Error>> {
  use v4l::video::Capture;
  use v4l::io::traits::CaptureStream;

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
  fmt.fourcc = v4l::FourCC::new(b"YUYV"); // https://stackoverflow.com/a/47736923
  // The camera we're using advertises the following color layout
  //   Raw       :     yuyv422 :           YUYV 4:2:2

  let assigned_fmt = dev.set_format(&fmt)?;

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

  let mut last_n_frame_times: [std::time::SystemTime; 8] = [std::time::SystemTime::now(); 8];
  // vv re-calculated off last_n_frame_times at regular intervals
  let mut rolling_fps_val: f32 = 0.0;

  let mut loop_i = 0;
  loop {
    loop_i += 1;
    if loop_i > 100000000 {
      loop_i = 0;
    }

    let (frame_mjpg_buf, meta) = stream.next()?;
    last_n_frame_times[loop_i % last_n_frame_times.len()] = std::time::SystemTime::now();
    if loop_i % 6 == 0 {
      rolling_fps_val = calc_fps_val(&last_n_frame_times);
      println!("rolling_fps_val = {:?}", rolling_fps_val);
    }


    // Exit if requested by another component
    if crate::PLEASE_EXIT_FLAG.load(std::sync::atomic::Ordering::Relaxed) {
      break;
    }
  }

  Ok(())
}



fn calc_fps_val(last_n_frame_times: &[std::time::SystemTime]) -> f32 {
  let mut frames_total_ms: f32 = 0.0;
  for i in 0..(last_n_frame_times.len()-1) {
    if let Ok(frame_t_dist) = last_n_frame_times[i+1].duration_since(last_n_frame_times[i]) {
      frames_total_ms += frame_t_dist.as_millis() as f32;
    }
  }
  let mut rolling_fps_val = last_n_frame_times.len() as f32 / frames_total_ms; // frames-per-millisecond
  rolling_fps_val *= 1000.0; // frames-per-second
  return rolling_fps_val;
}
