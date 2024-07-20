
pub async fn is_proc_running(proc_name: &str) -> bool {
  if let Ok(procs) = procfs::process::all_processes() {
    for p in procs {
      if let Ok(p) = p {
        if let Ok(p_exe) = p.exe() {
          if let Some(p_file_name) = p_exe.file_name() {
            let p_file_name = p_file_name.to_string_lossy();
            if p_file_name == proc_name {
              return true;
            }
          }
        }
      }
    }
  }
  return false;
}


pub async fn do_nice_shutdown() {

  crate::PLEASE_EXIT_FLAG.store(true, std::sync::atomic::Ordering::SeqCst);

  tokio::task::spawn(async { // Shutdown webserver after a 350ms delay to allow other tasks to exit
    tokio::time::sleep(tokio::time::Duration::from_millis(350)).await;
    if let Some(current_system) = actix_web::rt::System::try_current() {
      current_system.stop();
    }
  });

}


// Below is from https://gist.github.com/arifd/ea820ec97265a023e67a88b66955855d

// https://www.kernel.org/doc/html/v4.17/media/uapi/v4l/pixfmt-yuyv.html
//
// V4L2_PIX_FMT_YUYV — Packed format with ½ horizontal chroma resolution, also known as YUV 4:2:2
// Description
//
// In this format each four bytes is two pixels. Each four bytes is two Y's, a Cb and a Cr. Each Y goes to one of the pixels, and the Cb and Cr belong to both pixels. As you can see, the Cr and Cb components have half the horizontal resolution of the Y component. V4L2_PIX_FMT_YUYV is known in the Windows environment as YUY2.
//
// Example 2.19. V4L2_PIX_FMT_YUYV 4 × 4 pixel image
//
// Byte Order. Each cell is one byte.
// start + 0:  Y'00  Cb00  Y'01  Cr00  Y'02  Cb01  Y'03  Cr01
// start + 8:  Y'10  Cb10  Y'11  Cr10  Y'12  Cb11  Y'13  Cr11
// start + 16: Y'20  Cb20  Y'21  Cr20  Y'22  Cb21  Y'23  Cr21
// start + 24: Y'30  Cb30  Y'31  Cr30  Y'32  Cb31  Y'33  Cr31
//
// Color Sample Location.
//     0   1   2   3
// 0 Y C Y   Y C Y
// 1 Y C Y   Y C Y
// 2 Y C Y   Y C Y
// 3 Y C Y   Y C Y

use std::simd::f32x4;
use std::simd::num::SimdFloat;
use rayon::prelude::*;

/// Copies an input buffer of format YUYV422 to the output buffer
/// in the format of RGB24
// Interleaved: In case of YUV422 interleaved data, it looks as below:
//   Y1U1Y2V1 Y3U2Y4V2 ... ...
#[inline]
pub fn yuv422_interleaved_to_rgb24(in_buf: &[u8], out_buf: &mut [u8]) {
  debug_assert!(out_buf.len() as f32 == in_buf.len() as f32 * 1.5);

  in_buf
    .par_chunks_exact(4) // FIXME: use par_array_chunks() when stabalized (https://github.com/rayon-rs/rayon/pull/789)
    .zip(out_buf.par_chunks_exact_mut(6))
    .for_each(|(ch, out)| {
        let y1 = ch[0];
        let y2 = ch[2];
        let cb = ch[1];
        let cr = ch[3];

        let (r, g, b) = ycbcr_to_rgb(y1, cb, cr);

        out[0] = r;
        out[1] = g;
        out[2] = b;

        let (r, g, b) = ycbcr_to_rgb(y2, cb, cr);

        out[3] = r;
        out[4] = g;
        out[5] = b;
    });
}


// Planar: In memory, Y followed by Cb and followed by Cr
//   [Y1Y2......][Cb1Cb2......][Cr1Cr2.......]
#[inline]
pub fn yuv422_planar_to_rgb24(in_buf: &[u8], out_buf: &mut [u8]) {
  debug_assert!(out_buf.len() as f32 == in_buf.len() as f32 * 1.5);

  let y1y2_chunk_width = in_buf.len() / 2;
  let c_chunk_width = y1y2_chunk_width / 2;

  let cb_begin_idx = y1y2_chunk_width;
  let cr_begin_idx = y1y2_chunk_width + c_chunk_width;

  in_buf
    .par_chunks_exact(2) // iterate the 2 brightness components
    .zip(0..c_chunk_width) // add a component index offset so we can calculate an offset into in_buf
    .zip(out_buf.par_chunks_exact_mut(6))
    .for_each(|((ch, c_idx), out)| {
        let y1 = ch[0];
        let y2 = ch[1];
        let cb = in_buf[cb_begin_idx + c_idx];
        let cr = in_buf[cr_begin_idx + c_idx];

        let (r, g, b) = ycbcr_to_rgb(y1, cb, cr);

        out[0] = r;
        out[1] = g;
        out[2] = b;

        let (r, g, b) = ycbcr_to_rgb(y2, cb, cr);

        out[3] = r;
        out[4] = g;
        out[5] = b;
    });
}


// COLOR CONVERSION: https://stackoverflow.com/questions/28079010/rgb-to-ycbcr-using-simd-vectors-lose-some-data

#[inline]
fn ycbcr_to_rgb(y: u8, cb: u8, cr: u8) -> (u8, u8, u8) {
  let ycbcr = f32x4::from_array([y as f32, cb as f32 - 128.0f32, cr as f32 - 128.0f32, 0.0]);

  // rec 709: https://mymusing.co/bt-709-yuv-to-rgb-conversion-color/
  let r = (ycbcr * f32x4::from_array([1.0, 0.00000, 1.5748, 0.0])).reduce_sum();
  let g = (ycbcr * f32x4::from_array([1.0, -0.187324, -0.468124, 0.0])).reduce_sum();
  let b = (ycbcr * f32x4::from_array([1.0, 1.8556, 0.00000, 0.0])).reduce_sum();

  (clamp(r), clamp(g), clamp(b))
}

// fn rgb_to_ycbcr((r, g, b): (u8, u8, u8)) -> (u8, u8, u8) {
//     let rgb = F32x4(r as f32, g as f32, b as f32, 1.0);
//     let y = sum(mul(&rgb, F32x4(0.299000, 0.587000, 0.114000, 0.0)));
//     let cb = sum(mul(&rgb, F32x4(-0.168736, -0.331264, 0.500000, 128.0)));
//     let cr = sum(mul(&rgb, F32x4(0.500000, -0.418688, -0.081312, 128.0)));

//     (clamp(y), clamp(cb), clamp(cr))
// }

#[inline]
fn clamp(val: f32) -> u8 {
  if val < 0.0 {
      0
  } else if val > 255.0 {
      255
  } else {
      val.round() as u8
  }
}



