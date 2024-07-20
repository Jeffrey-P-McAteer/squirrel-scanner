
//pub static LAST_FRAME_PNG: [u8; 1280 * 720 * 3] = [0u8; 1280 * 720 * 3];
//pub static LAST_FRAME_PNG: &'static mut [u8] = &mut [0u8; 1280 * 720 * 3];
//pub static LAST_FRAME_PNG_BYTES_WRITTEN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

lazy_static::lazy_static! {
    pub static ref LAST_FRAME_PNG: Vec<u8> = vec![];
}


#[allow(unreachable_code)]
pub async fn camera_loop() -> Result<(), Box<dyn std::error::Error>> {
  use v4l::video::Capture;
  use v4l::io::traits::CaptureStream;

  use ffimage::color::Rgb;
  use ffimage::iter::PixelsExt;
  use ffimage::iter::ColorConvertExt;
  use ffimage::iter::BytesExt;
  use ffimage_yuv::{
      yuv::Yuv,
      yuv422::{Yuv422, Yuyv},
  };



  let mut video_device_path = "/dev/video2".to_string();
  if let Ok(val) = std::env::var("VDEV") {
    video_device_path = val.to_string();
  }
  println!("Using device {}", &video_device_path);

  let mut dev = v4l::Device::with_path(&video_device_path)?;

  // Let's say we want to explicitly request another format
  let mut fmt = dev.format()?;
  // fmt.width = 1920;
  // fmt.height = 1080;

  fmt.width = 1280;
  fmt.height = 720;

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
  println!("Camera cam_fmt_w,cam_fmt_h = {:?},{:?}", cam_fmt_w,cam_fmt_h);

  // To achieve the best possible performance, you may want to use a
  // UserBufferStream instance, but this is not supported on all devices,
  // so we stick to the mapped case for this example.
  // Please refer to the rustdoc docs for a more detailed explanation about
  // buffer transfers.

  // Create the stream, which will internally 'allocate' (as in map) the
  // number of requested buffers for us.
  let mut stream = v4l::io::mmap::Stream::with_buffers(&mut dev, v4l::buffer::Type::VideoCapture, 2)?;

  let mut last_n_frame_times: [std::time::SystemTime; 8] = [std::time::SystemTime::now(); 8];
  // vv re-calculated off last_n_frame_times at regular intervals
  let mut rolling_fps_val: f32;

  let mut loop_i = 0;
  loop {
    loop_i += 1;
    if loop_i > 100000000 {
      loop_i = 0;
    }

    let (frame_yuyv422_buf, meta) = stream.next()?;
    last_n_frame_times[loop_i % last_n_frame_times.len()] = std::time::SystemTime::now();
    if loop_i % 6 == 0 {
      rolling_fps_val = calc_fps_val(&last_n_frame_times);
      println!("rolling_fps_val = {:?}", rolling_fps_val);
    }

    // For now we're going to go ahead and do image processing inline.
    // At some point it may make sense to move this to another task
    // polling a queue, but for now this is nice and simple.
    {

      //let view = Image::<Yuv<u8>, _>::from_buf(&frame_yuyv422_buf, cam_fmt_w, cam_fmt_h)?;
      //let mut rgb_buf = Image::<Rgb<u8>, _>::new(cam_fmt_w, cam_fmt_h, 0u8);
      //view.convert(&mut rgb_buf);

      let mut rgb_pixels_buff: [u8; 1280 * 720 * 3] = [0u8; 1280 * 720 * 3];

      // YUV-to-RGB magic
      frame_yuyv422_buf.iter()
        .copied()
        .pixels::<Yuv<u8>>()
        .colorconvert::<Rgb<u8>>()
        .bytes()
        .write(&mut rgb_pixels_buff);


      let cam_fmt_w = cam_fmt_w as u32;
      let cam_fmt_h = cam_fmt_h as u32;

      let mut imgbuf = image::ImageBuffer::new(cam_fmt_w, cam_fmt_h);

      // Iterate over the coordinates and pixels of the image
      for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let rgb_buff_offset = ((y * cam_fmt_w) + x) as usize;
        *pixel = image::Rgb([
          rgb_pixels_buff[rgb_buff_offset + 0],
          rgb_pixels_buff[rgb_buff_offset + 1],
          rgb_pixels_buff[rgb_buff_offset + 2]
        ]);
      }


      if let Err(e) = imgbuf.save("/tmp/img.png") {
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
