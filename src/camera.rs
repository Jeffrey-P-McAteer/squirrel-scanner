
static CAMERA_ROLLING_FPS: atomic_float::AtomicF32 = atomic_float::AtomicF32::new(0.0);
lazy_static::lazy_static! {
  pub static ref CAMERA_LAST_FRAME_JPEG: std::sync::RwLock<Vec<u8>> = std::sync::RwLock::new(vec![]);
  pub static ref CAMERA_LAST_FRAME_IMGBUF: std::sync::RwLock<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>> = std::sync::RwLock::new(image::ImageBuffer::new(1, 1));
}

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

  let (frame_tx, frame_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(2);

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
  let mut stream = v4l::io::mmap::Stream::with_buffers(&mut dev, v4l::buffer::Type::VideoCapture, 2)?;

  let mut last_n_frame_times: [std::time::SystemTime; 8] = [std::time::SystemTime::now(); 8];

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
      let is_ignorable = e.to_string().contains("SendError");
      if !is_ignorable {
        eprintln!("[ frame_tx.send ] {:?} (to_string={})", e, e.to_string());
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


#[allow(unreachable_code)]
pub async fn run_frame_processor(cam_fmt_w: usize, cam_fmt_h: usize, mut frame_rx: tokio::sync::mpsc::Receiver<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
  use image::GenericImageView;
  // frame_rx produces raw YUV vectors of image data

  let font = ab_glyph::FontRef::try_from_slice(include_bytes!("/usr/share/fonts/noto/NotoSansMono-Regular.ttf"))?;

  // If model file does not exist, download it first!
  const YOLOV8M_DOWNLOAD_URL: &'static str = "https://raw.githubusercontent.com/AndreyGermanov/yolov8_onnx_rust/main/yolov8m.onnx";
  const YOLOV8M_LOCAL_FILE: &'static str = "/tmp/yolov8m.onnx";
  if !std::path::Path::new(YOLOV8M_LOCAL_FILE).exists() {
    let resp = reqwest::get(YOLOV8M_DOWNLOAD_URL).await?;
    let body = resp.bytes().await?;
    let mut out = std::fs::File::create(YOLOV8M_LOCAL_FILE)?;
    let body = body.to_vec();
    std::io::copy(&mut body[..].as_ref(), &mut out)?;
  }

  let mut rgb_pixels_buff: Vec<u8> = vec![0u8; cam_fmt_w * cam_fmt_h * 3];

  // See https://github.com/pykeio/ort/blob/main/examples/yolov8/examples/yolov8.rs
  let model = ort::Session::builder()?.commit_from_file(YOLOV8M_LOCAL_FILE)?;

  // Finally we also dump pixels to /dev/fb0 for funsies
  let mut framebuffer_path = "/dev/fb0".to_string();
  if let Ok(val) = std::env::var("FB") {
    framebuffer_path = val.to_string();
  }
  println!("Using framebuffer device {}", &framebuffer_path);
  let mut maybe_framebuffer = framebuffer::Framebuffer::new(&framebuffer_path[..]);
  let mut fb_w = 1;
  let mut fb_h = 1;
  let mut fb_line_length = 1;
  let mut fb_bytespp = 1;
  if let Ok(ref mut fb) = maybe_framebuffer {
    if let Err(e) = framebuffer::Framebuffer::set_kd_mode(framebuffer::KdMode::Graphics) {
      eprintln!("[ Framebuffer::set_kd_mode Graphics ] {:?}", e);
    }
    fb_w = fb.var_screen_info.xres;
    fb_h = fb.var_screen_info.yres;
    fb_line_length = fb.fix_screen_info.line_length;
    fb_bytespp = fb.var_screen_info.bits_per_pixel / 8;
  }
  let mut fb_frame = vec![0u8; (fb_line_length * fb_h) as usize];
  println!("fb_w={} fb_h={} fb_bytespp={} fb_line_length={}", fb_w, fb_h, fb_bytespp, fb_line_length);

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

      {
        // We convert the image to a smaller copy, run the ONNX runtime over it,
        // then convert small-space detection coordinates back to the original image size
        // and use _that_ to draw overlays etc.
        let onx_imgbuf = image::DynamicImage::ImageRgb8(image::RgbImage::from_raw(cam_fmt_w, cam_fmt_h, rgb_pixels_buff.clone() ).expect("Cannot read RGB image!") );
        let onx_imgbuf = onx_imgbuf.resize_exact(640, 640, image::imageops::FilterType::CatmullRom);
        let mut input = ndarray::Array::zeros((1, 3, 640, 640)).into_dyn();
        for pixel in onx_imgbuf.pixels() {
            let x = pixel.0 as usize;
            let y = pixel.1 as usize;
            let [r,g,b,_] = pixel.2.0;
            input[[0, 0, y, x]] = (r as f32) / 255.0;
            input[[0, 1, y, x]] = (g as f32) / 255.0;
            input[[0, 2, y, x]] = (b as f32) / 255.0;
        };

        let outputs: ort::SessionOutputs = model.run(ort::inputs!["images" => input.view()]?)?;
        let output = outputs["output0"].try_extract_tensor::<f32>()?.t().into_owned();

        let mut boxes = Vec::new();
        let output = output.slice(ndarray::s![.., .., 0]);
        for row in output.axis_iter(ndarray::Axis(0)) {
          let row: Vec<_> = row.iter().copied().collect();
          let (class_id, prob) = row
            .iter()
            // skip bounding box coordinates
            .skip(4)
            .enumerate()
            .map(|(index, value)| (index, *value))
            .reduce(|accum, row| if row.1 > accum.1 { row } else { accum })
            .unwrap();
          if prob < 0.5 {
            continue;
          }
          let label = YOLOV8_CLASS_LABELS[class_id];
          let xc = row[0] / 640. * (cam_fmt_w as f32);
          let yc = row[1] / 640. * (cam_fmt_h as f32);
          let w = row[2] / 640. * (cam_fmt_w as f32);
          let h = row[3] / 640. * (cam_fmt_h as f32);
          boxes.push((
            BoundingBox {
              x1: xc - w / 2.,
              y1: yc - h / 2.,
              x2: xc + w / 2.,
              y2: yc + h / 2.
            },
            label,
            prob
          ));
        }

        boxes.sort_by(|box1, box2| box2.2.total_cmp(&box1.2));
        let mut result = Vec::new();

        while !boxes.is_empty() {
          result.push(boxes[0]);
          boxes = boxes
            .iter()
            .filter(|box1| intersection(&boxes[0].0, &box1.0) / union(&boxes[0].0, &box1.0) < 0.7)
            .copied()
            .collect();
        }

        // Draw some debug text along the bottom
        let dbg_text = format!("result = {:?}", result);
        println!("{}", dbg_text);
        imageproc::drawing::draw_text_mut(
          &mut imgbuf,
          image::Rgb([255, 255, 255]),
          4, cam_fmt_h as i32 - 72, ab_glyph::PxScale::from(18.0),
          &font,
          &dbg_text[..]
        );

        // Now draw the identifying box markers
        for (bbox, label, confidence) in result.iter() {
          imageproc::drawing::draw_hollow_rect_mut(
            &mut imgbuf,
            imageproc::rect::Rect::at(bbox.x1 as i32, bbox.y1 as i32).of_size((bbox.x2 - bbox.x1) as u32, (bbox.y2 - bbox.y1) as u32),
            image::Rgb([176, 176, 176]),
          );

          let label_txt = format!("{} ({:.1})", label, confidence);
          imageproc::drawing::draw_text_mut(
            &mut imgbuf,
            image::Rgb([255, 255, 255]),
            bbox.x1 as i32 + 6, bbox.y1 as i32 + 8, ab_glyph::PxScale::from(18.0),
            &font,
            &label_txt[..]
          );
        }
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

      // DONE drawing!
      let imgbuf = imgbuf;

      // If we have an open framebuffer, write imgbuf data to it
      if let Ok(ref mut fb) = maybe_framebuffer {
        println!("Writing to framebuffer!");
        for (x, y, px) in imgbuf.enumerate_pixels() {
            let start_index = (((y * fb_line_length) + x) * fb_bytespp) as usize;
            fb_frame[start_index] = px.0[0]; // assume R
            fb_frame[start_index + 1] = px.0[1]; // Assume G
            fb_frame[start_index + 2] = px.0[2]; // Assume B
        }

        fb.write_frame(&fb_frame);

      }


      if let Ok(mut camera_last_frame_jpeg) = CAMERA_LAST_FRAME_JPEG.write() {
        let mut seekable_buf: std::io::Cursor<Vec<u8>> = std::io::Cursor::new( Vec::new() );
        if let Err(e) = imgbuf.write_to(&mut seekable_buf, image::ImageFormat::Jpeg) {
          eprintln!("[ imgbuf.write_to ] {:?}", e);
        }
        *camera_last_frame_jpeg = seekable_buf.into_inner();
      }

      if let Ok(mut camera_last_frame_imgbuf) = CAMERA_LAST_FRAME_IMGBUF.write() {
        *camera_last_frame_imgbuf = imgbuf;
      }

    }

    // Exit if requested by another component
    if crate::PLEASE_EXIT_FLAG.load(std::sync::atomic::Ordering::Relaxed) {
      break;
    }
  }

  // Important: re-enable text mode!
  if let Err(e) = framebuffer::Framebuffer::set_kd_mode(framebuffer::KdMode::Text) {
    eprintln!("[ Framebuffer::set_kd_mode Text ] {:?}", e);
  }

  Ok(())
}




#[rustfmt::skip]
const YOLOV8_CLASS_LABELS: [&str; 80] = [
    "person", "bicycle", "car", "motorcycle", "airplane", "bus", "train", "truck", "boat", "traffic light",
  "fire hydrant", "stop sign", "parking meter", "bench", "bird", "cat", "dog", "horse", "sheep", "cow", "elephant",
  "bear", "zebra", "giraffe", "backpack", "umbrella", "handbag", "tie", "suitcase", "frisbee", "skis", "snowboard",
  "sports ball", "kite", "baseball bat", "baseball glove", "skateboard", "surfboard", "tennis racket", "bottle",
  "wine glass", "cup", "fork", "knife", "spoon", "bowl", "banana", "apple", "sandwich", "orange", "broccoli",
  "carrot", "hot dog", "pizza", "donut", "cake", "chair", "couch", "potted plant", "bed", "dining table", "toilet",
  "tv", "laptop", "mouse", "remote", "keyboard", "cell phone", "microwave", "oven", "toaster", "sink", "refrigerator",
  "book", "clock", "vase", "scissors", "teddy bear", "hair drier", "toothbrush"
];

#[derive(Debug, Clone, Copy)]
struct BoundingBox {
  x1: f32,
  y1: f32,
  x2: f32,
  y2: f32
}

fn intersection(box1: &BoundingBox, box2: &BoundingBox) -> f32 {
  (box1.x2.min(box2.x2) - box1.x1.max(box2.x1)) * (box1.y2.min(box2.y2) - box1.y1.max(box2.y1))
}

fn union(box1: &BoundingBox, box2: &BoundingBox) -> f32 {
  ((box1.x2 - box1.x1) * (box1.y2 - box1.y1)) + ((box2.x2 - box2.x1) * (box2.y2 - box2.y1)) - intersection(box1, box2)
}
