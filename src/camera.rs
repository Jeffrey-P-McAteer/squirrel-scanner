
static CAMERA_ROLLING_FPS: atomic_float::AtomicF32 = atomic_float::AtomicF32::new(0.0);

#[allow(unreachable_code)]
pub async fn camera_loop() -> Result<(), Box<dyn std::error::Error>> {
  use v4l::video::Capture;
  use v4l::io::traits::CaptureStream;

  //let font = ab_glyph::FontRef::try_from_slice(include_bytes!("/usr/share/fonts/noto/NotoSansMono-Regular.ttf"))?;

  let mut video_device_path = "/dev/video2".to_string();
  if let Ok(val) = std::env::var("VDEV") {
    video_device_path = val.to_string();
  }
  println!("Using device {}", &video_device_path);

  let mut dev = v4l::Device::with_path(&video_device_path)?;

  // Let's say we want to explicitly request another format
  let mut fmt = dev.format()?;

  for (wanted_w, wanted_h) in &[(1920, 1080), (1280, 720)] {
    fmt.fourcc = v4l::FourCC::new(b"YUYV");
    fmt.width = *wanted_w;
    fmt.height = *wanted_h;
    let assigned_fmt = dev.set_format(&fmt)?;
    if assigned_fmt.width == *wanted_w && assigned_fmt.height == *wanted_h {
      break;
    }
  }

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
  println!("Camera cam_fmt_w,cam_fmt_h = {:?},{:?}", cam_fmt_w,cam_fmt_h);

  let (frame_tx, mut frame_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(16);

  let task_cam_fmt_w = cam_fmt_w;
  let task_cam_fmt_h = cam_fmt_h;
  tokio::task::spawn(async move {
    if let Err(e) = run_frame_processor(task_cam_fmt_w, task_cam_fmt_h, frame_rx).await {
      eprintln!("[ run_frame_processor ] {:?}", e);
    }
    // If the frame processor goes down, everything should be going down.
    crate::utils::do_nice_shutdown().await;
  });

  // Create the stream, which will internally 'allocate' (as in map) the
  // number of requested buffers for us.
  let mut stream = v4l::io::mmap::Stream::with_buffers(&mut dev, v4l::buffer::Type::VideoCapture, 4)?;

  let mut last_n_frame_times: [std::time::SystemTime; 8] = [std::time::SystemTime::now(); 8];
  // vv re-calculated off last_n_frame_times at regular intervals
  // let mut rolling_fps_val: f32 = 0.0;

  //let mut rgb_pixels_buff: Vec<u8> = vec![0u8; cam_fmt_w * cam_fmt_h * 3];

  let mut loop_i = 0;
  loop {
    loop_i += 1;
    if loop_i > 100000000 {
      loop_i = 0;
    }

    let (frame_yuyv422_buf, meta) = stream.next()?;
    last_n_frame_times[loop_i % last_n_frame_times.len()] = std::time::SystemTime::now();
    if loop_i % 6 == 0 {
      let rolling_fps_val = calc_fps_val(&last_n_frame_times);
      CAMERA_ROLLING_FPS.store(rolling_fps_val, std::sync::atomic::Ordering::Relaxed);
      println!("rolling_fps_val = {:?}", rolling_fps_val);
    }

    if let Err(e) = frame_tx.send( frame_yuyv422_buf.to_vec() ).await {
      eprintln!("[ frame_tx.send ] {:?}", e);
    }

    /*
    // For now we're going to go ahead and do image processing inline.
    // At some point it may make sense to move this to another task
    // polling a queue, but for now this is nice and simple.
    {
      crate::utils::yuv422_interleaved_to_rgb24(&frame_yuyv422_buf, &mut rgb_pixels_buff[..]);

      let cam_fmt_w = cam_fmt_w as u32;
      let cam_fmt_h = cam_fmt_h as u32;

      let mut imgbuf = image::ImageBuffer::new(cam_fmt_w, cam_fmt_h);

      // Iterate over the coordinates and pixels of the image
      for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let rgb_buff_offset = ((y * cam_fmt_w * 3) + (x * 3) ) as usize;
        *pixel = image::Rgb([
          rgb_pixels_buff[rgb_buff_offset + 0],
          rgb_pixels_buff[rgb_buff_offset + 1],
          rgb_pixels_buff[rgb_buff_offset + 2]
        ]);
      }

      let now = chrono::Local::now();
      let ts_text = format!("{} FPS={:.1}", now.format("%H:%M:%S"), rolling_fps_val);
      imageproc::drawing::draw_text_mut(
        &mut imgbuf,
        image::Rgb([255, 255, 255]),
        4, 4, ab_glyph::PxScale::from(18.0),
        &font,
        &ts_text[..]
      );

      if let Err(e) = imgbuf.save("/tmp/img.jpg") {
        eprintln!("[ imgbuf.save ] {:?}", e);
      }

    }
  */

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


#[allow(unreachable_code)]
pub async fn run_frame_processor(cam_fmt_w: usize, cam_fmt_h: usize, mut frame_rx: tokio::sync::mpsc::Receiver<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
  // frame_rx produces raw YUV vectors of image data

  let font = ab_glyph::FontRef::try_from_slice(include_bytes!("/usr/share/fonts/noto/NotoSansMono-Regular.ttf"))?;

  let mut rgb_pixels_buff: Vec<u8> = vec![0u8; cam_fmt_w * cam_fmt_h * 3];

  loop {

    if let Some(frame_yuyv422_buf) = frame_rx.recv().await {
      crate::utils::yuv422_interleaved_to_rgb24(&frame_yuyv422_buf[..], &mut rgb_pixels_buff[..]);

      let cam_fmt_w = cam_fmt_w as u32;
      let cam_fmt_h = cam_fmt_h as u32;

      let mut imgbuf = image::ImageBuffer::new(cam_fmt_w, cam_fmt_h);

      // Iterate over the coordinates and pixels of the image
      for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let rgb_buff_offset = ((y * cam_fmt_w * 3) + (x * 3) ) as usize;
        *pixel = image::Rgb([
          rgb_pixels_buff[rgb_buff_offset + 0],
          rgb_pixels_buff[rgb_buff_offset + 1],
          rgb_pixels_buff[rgb_buff_offset + 2]
        ]);
      }

      let rolling_fps_val = CAMERA_ROLLING_FPS.load(std::sync::atomic::Ordering::Relaxed);
      let now = chrono::Local::now();
      let ts_text = format!("{} FPS={:.1}", now.format("%H:%M:%S"), rolling_fps_val);
      imageproc::drawing::draw_text_mut(
        &mut imgbuf,
        image::Rgb([255, 255, 255]),
        4, 4, ab_glyph::PxScale::from(18.0),
        &font,
        &ts_text[..]
      );

      if let Err(e) = imgbuf.save("/tmp/img.jpg") {
        eprintln!("[ imgbuf.save ] {:?}", e);
      }
    }

    // Exit if requested by another component
    if crate::PLEASE_EXIT_FLAG.load(std::sync::atomic::Ordering::Relaxed) {
      break;
    }
  }

  Ok(())
}
