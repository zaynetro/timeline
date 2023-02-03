use std::io::Cursor;

use anyhow::{bail, Result};
use image::io::Reader as ImageReader;
use image::ImageFormat;
use image::{DynamicImage, GrayImage, RgbaImage};

use crate::api::PixelFormat;

pub fn scan_qr_code(
    width: u32,
    height: u32,
    format: PixelFormat,
    buf: Vec<u8>,
) -> Result<Option<String>> {
    let img = match format {
        PixelFormat::BGRA8888 => read_bgra8888(width, height, buf)?,
        PixelFormat::JPEG => read_jpeg(buf)?,
    };
    find_qr_code(img.to_luma8())
}

fn find_qr_code(img: GrayImage) -> Result<Option<String>> {
    let mut img = rqrr::PreparedImage::prepare(img);
    let grids = img.detect_grids();
    if grids.is_empty() {
        Ok(None)
    } else {
        let (_meta, content) = grids[0].decode()?;
        Ok(Some(content))
    }
}

// https://users.rust-lang.org/t/converting-a-bgra-u8-to-rgb-u8-n-for-images/67938/8?u=cad97
fn convert_bgra(width: u32, height: u32, mut bgra: Vec<u8>) -> Option<RgbaImage> {
    for src in bgra.chunks_exact_mut(4) {
        let (blue, green, red, alpha) = (src[0], src[1], src[2], src[3]);
        src[0] = red;
        src[1] = green;
        src[2] = blue;
        src[3] = alpha;
    }
    RgbaImage::from_raw(width, height, bgra)
}

fn read_bgra8888(width: u32, height: u32, buf: Vec<u8>) -> Result<DynamicImage> {
    let buf_len = buf.len();
    if buf_len % 4 != 0 {
        bail!("Incorrect buf len={} for BGRA8888", buf.len());
    }
    let Some(img) = convert_bgra(width, height, buf) else {
        bail!("Incorrect buf len={} for BGRA8888", buf_len);
    };
    Ok(DynamicImage::ImageRgba8(img))
}

fn read_jpeg(buf: Vec<u8>) -> Result<DynamicImage> {
    let img = ImageReader::with_format(Cursor::new(buf), ImageFormat::Jpeg).decode()?;
    Ok(img)
}
