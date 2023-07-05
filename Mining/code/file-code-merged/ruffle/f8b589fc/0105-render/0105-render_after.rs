use crate::shape_utils::DistilledShape;
pub use crate::{library::MovieLibrary, transform::Transform, Color};
use downcast_rs::Downcast;
use gc_arena::Collect;
use std::io::Read;
pub use swf;
use swf::Matrix;

pub trait RenderBackend: Downcast {
    fn set_viewport_dimensions(&mut self, width: u32, height: u32);
    fn register_shape(
        &mut self,
        shape: DistilledShape,
        library: Option<&MovieLibrary<'_>>,
    ) -> ShapeHandle;
    fn replace_shape(
        &mut self,
        shape: DistilledShape,
        library: Option<&MovieLibrary<'_>>,
        handle: ShapeHandle,
    );
    fn register_glyph_shape(&mut self, shape: &swf::Glyph) -> ShapeHandle;
    fn register_bitmap_jpeg(
        &mut self,
        data: &[u8],
        jpeg_tables: Option<&[u8]>,
    ) -> Result<BitmapInfo, Error>;
    fn register_bitmap_jpeg_2(&mut self, data: &[u8]) -> Result<BitmapInfo, Error>;
    fn register_bitmap_jpeg_3(
        &mut self,
        jpeg_data: &[u8],
        alpha_data: &[u8],
    ) -> Result<BitmapInfo, Error>;
    fn register_bitmap_png(
        &mut self,
        swf_tag: &swf::DefineBitsLossless,
    ) -> Result<BitmapInfo, Error>;

    fn begin_frame(&mut self, clear: Color);
    fn render_bitmap(&mut self, bitmap: BitmapHandle, transform: &Transform, smoothing: bool);
    fn render_shape(&mut self, shape: ShapeHandle, transform: &Transform);
    fn draw_rect(&mut self, color: Color, matrix: &Matrix);
    fn end_frame(&mut self);
    fn push_mask(&mut self);
    fn activate_mask(&mut self);
    fn deactivate_mask(&mut self);
    fn pop_mask(&mut self);

    fn get_bitmap_pixels(&mut self, bitmap: BitmapHandle) -> Option<Bitmap>;
    fn register_bitmap_raw(
        &mut self,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) -> Result<BitmapHandle, Error>;
    fn update_texture(
        &mut self,
        bitmap: BitmapHandle,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) -> Result<BitmapHandle, Error>;
}
impl_downcast!(RenderBackend);

type Error = Box<dyn std::error::Error>;

#[derive(Copy, Clone, Debug)]
pub struct ShapeHandle(pub usize);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Collect)]
#[collect(no_drop)]
pub struct BitmapHandle(pub usize);

/// Info returned by the `register_bitmap` methods.
#[derive(Copy, Clone, Debug)]
pub struct BitmapInfo {
    pub handle: BitmapHandle,
    pub width: u16,
    pub height: u16,
}

pub struct NullRenderer;

impl NullRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NullRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderBackend for NullRenderer {
    fn set_viewport_dimensions(&mut self, _width: u32, _height: u32) {}
    fn register_shape(
        &mut self,
        _shape: DistilledShape,
        _library: Option<&MovieLibrary<'_>>,
    ) -> ShapeHandle {
        ShapeHandle(0)
    }
    fn replace_shape(
        &mut self,
        _shape: DistilledShape,
        _library: Option<&MovieLibrary<'_>>,
        _handle: ShapeHandle,
    ) {
    }
    fn register_glyph_shape(&mut self, _shape: &swf::Glyph) -> ShapeHandle {
        ShapeHandle(0)
    }
    fn register_bitmap_jpeg(
        &mut self,
        _data: &[u8],
        _jpeg_tables: Option<&[u8]>,
    ) -> Result<BitmapInfo, Error> {
        Ok(BitmapInfo {
            handle: BitmapHandle(0),
            width: 0,
            height: 0,
        })
    }
    fn register_bitmap_jpeg_2(&mut self, _data: &[u8]) -> Result<BitmapInfo, Error> {
        Ok(BitmapInfo {
            handle: BitmapHandle(0),
            width: 0,
            height: 0,
        })
    }
    fn register_bitmap_jpeg_3(
        &mut self,
        _data: &[u8],
        _alpha_data: &[u8],
    ) -> Result<BitmapInfo, Error> {
        Ok(BitmapInfo {
            handle: BitmapHandle(0),
            width: 0,
            height: 0,
        })
    }
    fn register_bitmap_png(
        &mut self,
        _swf_tag: &swf::DefineBitsLossless,
    ) -> Result<BitmapInfo, Error> {
        Ok(BitmapInfo {
            handle: BitmapHandle(0),
            width: 0,
            height: 0,
        })
    }
    fn begin_frame(&mut self, _clear: Color) {}
    fn end_frame(&mut self) {}
    fn render_bitmap(&mut self, _bitmap: BitmapHandle, _transform: &Transform, _smoothing: bool) {}
    fn render_shape(&mut self, _shape: ShapeHandle, _transform: &Transform) {}
    fn draw_rect(&mut self, _color: Color, _matrix: &Matrix) {}
    fn push_mask(&mut self) {}
    fn activate_mask(&mut self) {}
    fn deactivate_mask(&mut self) {}
    fn pop_mask(&mut self) {}

    fn get_bitmap_pixels(&mut self, _bitmap: BitmapHandle) -> Option<Bitmap> {
        None
    }
    fn register_bitmap_raw(
        &mut self,
        _width: u32,
        _height: u32,
        _rgba: Vec<u8>,
    ) -> Result<BitmapHandle, Error> {
        Ok(BitmapHandle(0))
    }

    fn update_texture(
        &mut self,
        _bitmap: BitmapHandle,
        _width: u32,
        _height: u32,
        _rgba: Vec<u8>,
    ) -> Result<BitmapHandle, Error> {
        Ok(BitmapHandle(0))
    }
}

/// The format of image data in a DefineBitsJpeg2/3 tag.
/// Generally this will be JPEG, but according to SWF19, these tags can also contain PNG and GIF data.
/// SWF19 pp.138-139
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum JpegTagFormat {
    Jpeg,
    Png,
    Gif,
    Unknown,
}

/// Decoded bitmap data from an SWF tag.
#[derive(Debug, Clone)]
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub data: BitmapFormat,
}

/// Decoded bitmap data from an SWF tag.
/// The image data will have pre-multiplied alpha.
#[derive(Debug, Clone)]
pub enum BitmapFormat {
    Rgb(Vec<u8>),
    Rgba(Vec<u8>),
}

impl From<BitmapFormat> for Vec<i32> {
    fn from(format: BitmapFormat) -> Self {
        match format {
            BitmapFormat::Rgb(x) => x
                .chunks_exact(3)
                .map(|chunk| {
                    let red = chunk[0];
                    let green = chunk[1];
                    let blue = chunk[2];
                    (0xFF << 24) | ((red as i32) << 16) | ((green as i32) << 8) | (blue as i32)
                })
                .collect(),
            BitmapFormat::Rgba(x) => x
                .chunks_exact(4)
                .map(|chunk| {
                    let red = chunk[0];
                    let green = chunk[1];
                    let blue = chunk[2];
                    let alpha = chunk[3];
                    ((alpha as i32) << 24)
                        | ((red as i32) << 16)
                        | ((green as i32) << 8)
                        | (blue as i32)
                })
                .collect(),
        }
    }
}

/// Determines the format of the image data in `data` from a DefineBitsJPEG2/3 tag.
pub fn determine_jpeg_tag_format(data: &[u8]) -> JpegTagFormat {
    match data {
        [0xff, 0xd8, ..] => JpegTagFormat::Jpeg,
        [0xff, 0xd9, 0xff, 0xd8, ..] => JpegTagFormat::Jpeg, // erroneous header in SWF
        [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, ..] => JpegTagFormat::Png,
        [0x47, 0x49, 0x46, 0x38, 0x39, 0x61, ..] => JpegTagFormat::Gif,
        _ => JpegTagFormat::Unknown,
    }
}

/// Decodes bitmap data from a DefineBitsJPEG2/3 tag.
/// The data is returned with pre-multiplied alpha.
pub fn decode_define_bits_jpeg(data: &[u8], alpha_data: Option<&[u8]>) -> Result<Bitmap, Error> {
    let format = determine_jpeg_tag_format(data);
    if format != JpegTagFormat::Jpeg && alpha_data.is_some() {
        // Only DefineBitsJPEG3 with true JPEG data should have separate alpha data.
        log::warn!("DefineBitsJPEG contains non-JPEG data with alpha; probably incorrect")
    }
    match format {
        JpegTagFormat::Jpeg => decode_jpeg(data, alpha_data),
        JpegTagFormat::Png => decode_png(data),
        JpegTagFormat::Gif => decode_gif(data),
        JpegTagFormat::Unknown => Err("Unknown bitmap data format".into()),
    }
}

pub fn glue_swf_jpeg_to_tables(jpeg_tables: &[u8], jpeg_data: &[u8]) -> Vec<u8> {
    let mut full_jpeg = Vec::with_capacity(jpeg_tables.len() + jpeg_data.len() - 4);
    full_jpeg.extend_from_slice(&jpeg_tables[..jpeg_tables.len() - 2]);
    full_jpeg.extend_from_slice(&jpeg_data[2..]);
    full_jpeg
}

/// Glues the JPEG encoding tables from a JPEGTables SWF tag to the JPEG data
/// in a DefineBits tag, producing complete JPEG data suitable for a decoder.
pub fn glue_tables_to_jpeg<'a>(
    jpeg_data: &'a [u8],
    jpeg_tables: Option<&'a [u8]>,
) -> std::borrow::Cow<'a, [u8]> {
    if let Some(jpeg_tables) = jpeg_tables {
        if jpeg_tables.len() >= 2 {
            let mut full_jpeg = Vec::with_capacity(jpeg_tables.len() + jpeg_data.len());
            full_jpeg.extend_from_slice(&jpeg_tables[..jpeg_tables.len() - 2]);
            if jpeg_data.len() >= 2 {
                full_jpeg.extend_from_slice(&jpeg_data[2..]);
            }

            return std::borrow::Cow::from(full_jpeg);
        }
    }

    // No JPEG tables or not enough data; return JPEG data as is
    std::borrow::Cow::Borrowed(jpeg_data)
}

/// Removes potential invalid JPEG data from SWF DefineBitsJPEG tags.
///
/// SWF19 p.138:
/// "Before version 8 of the SWF file format, SWF files could contain an erroneous header of 0xFF, 0xD9, 0xFF, 0xD8 before the JPEG SOI marker."
/// These bytes need to be removed for the JPEG to decode properly.
pub fn remove_invalid_jpeg_data(mut data: &[u8]) -> std::borrow::Cow<[u8]> {
    // TODO: Might be better to return an Box<Iterator<Item=u8>> instead of a Cow here,
    // where the spliced iter is a data[..n].chain(data[n+4..])?
    if data.get(0..4) == Some(&[0xFF, 0xD9, 0xFF, 0xD8]) {
        data = &data[4..];
    }
    if let Some(pos) = data.windows(4).position(|w| w == [0xFF, 0xD9, 0xFF, 0xD8]) {
        let mut out_data = Vec::with_capacity(data.len() - 4);
        out_data.extend_from_slice(&data[..pos]);
        out_data.extend_from_slice(&data[pos + 4..]);
        std::borrow::Cow::from(out_data)
    } else {
        std::borrow::Cow::Borrowed(data)
    }
}

/// Decodes a JPEG with optional alpha data.
/// The decoded bitmap will have pre-multiplied alpha.
pub fn decode_jpeg(
    jpeg_data: &[u8],
    alpha_data: Option<&[u8]>,
) -> Result<Bitmap, Box<dyn std::error::Error>> {
    let jpeg_data = remove_invalid_jpeg_data(jpeg_data);

    let mut decoder = jpeg_decoder::Decoder::new(&jpeg_data[..]);
    decoder.read_info()?;
    let metadata = decoder.info().ok_or("Unable to get image info")?;
    let decoded_data = decoder.decode()?;

    let decoded_data = match metadata.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => decoded_data,
        jpeg_decoder::PixelFormat::CMYK32 => {
            log::warn!("Unimplemented CMYK32 JPEG pixel format");
            decoded_data
        }
        jpeg_decoder::PixelFormat::L8 => {
            let mut rgb = Vec::with_capacity(decoded_data.len() * 3);
            for elem in decoded_data {
                rgb.push(elem);
                rgb.push(elem);
                rgb.push(elem);
            }
            rgb
        }
    };

    // Decompress the alpha data (DEFLATE compression).
    if let Some(alpha_data) = alpha_data {
        let alpha_data = decompress_zlib(alpha_data)?;

        if alpha_data.len() == decoded_data.len() / 3 {
            let mut rgba = Vec::with_capacity((decoded_data.len() / 3) * 4);
            let mut i = 0;
            let mut a = 0;
            while i < decoded_data.len() {
                rgba.push(decoded_data[i]);
                rgba.push(decoded_data[i + 1]);
                rgba.push(decoded_data[i + 2]);
                rgba.push(alpha_data[a]);
                i += 3;
                a += 1;
            }
            return Ok(Bitmap {
                width: metadata.width.into(),
                height: metadata.height.into(),
                data: BitmapFormat::Rgba(rgba),
            });
        } else {
            // Size isn't correct; fallback to RGB?
            log::error!("Size mismatch in DefineBitsJPEG3 alpha data");
        }
    }

    // No alpha.
    Ok(Bitmap {
        width: metadata.width.into(),
        height: metadata.height.into(),
        data: BitmapFormat::Rgb(decoded_data),
    })
}

fn rgb5_component(compressed: u16, shift: u16) -> u8 {
    let component = compressed >> shift & 0x1F;
    ((component * 255 + 15) / 31) as u8
}

/// Decodes the bitmap data in DefineBitsLossless tag into RGBA.
/// DefineBitsLossless is Zlib encoded pixel data (similar to PNG), possibly
/// palletized.
pub fn decode_define_bits_lossless(
    swf_tag: &swf::DefineBitsLossless,
) -> Result<Bitmap, Box<dyn std::error::Error>> {
    // Decompress the image data (DEFLATE compression).
    let mut decoded_data = decompress_zlib(swf_tag.data)?;

    // Swizzle/de-palettize the bitmap.
    let out_data = match (swf_tag.version, swf_tag.format) {
        (1, swf::BitmapFormat::Rgb15) => {
            let padded_width = (swf_tag.width + 0b1) & !0b1;
            let mut out_data: Vec<u8> =
                Vec::with_capacity(swf_tag.width as usize * swf_tag.height as usize * 4);
            let mut i = 0;
            for _ in 0..swf_tag.height {
                for _ in 0..swf_tag.width {
                    let compressed = ((decoded_data[i] as u16) << 8) | decoded_data[i + 1] as u16;
                    out_data.push(rgb5_component(compressed, 10));
                    out_data.push(rgb5_component(compressed, 5));
                    out_data.push(rgb5_component(compressed, 0));
                    out_data.push(0xff);
                    i += 2;
                }
                i += (padded_width - swf_tag.width) as usize * 2;
            }
            out_data
        }
        (1, swf::BitmapFormat::Rgb32) => {
            let mut i = 0;
            while i < decoded_data.len() {
                decoded_data[i] = decoded_data[i + 1];
                decoded_data[i + 1] = decoded_data[i + 2];
                decoded_data[i + 2] = decoded_data[i + 3];
                decoded_data[i + 3] = 0xff;
                i += 4;
            }
            decoded_data
        }
        (2, swf::BitmapFormat::Rgb32) => {
            let mut i = 0;
            while i < decoded_data.len() {
                let alpha = decoded_data[i];
                decoded_data[i] = decoded_data[i + 1];
                decoded_data[i + 1] = decoded_data[i + 2];
                decoded_data[i + 2] = decoded_data[i + 3];
                decoded_data[i + 3] = alpha;
                i += 4;
            }
            decoded_data
        }
        (1, swf::BitmapFormat::ColorMap8) => {
            let mut i = 0;
            let padded_width = (swf_tag.width + 0b11) & !0b11;

            let mut palette = Vec::with_capacity(swf_tag.num_colors as usize + 1);
            for _ in 0..=swf_tag.num_colors {
                palette.push(Color {
                    r: decoded_data[i],
                    g: decoded_data[i + 1],
                    b: decoded_data[i + 2],
                    a: 255,
                });
                i += 3;
            }
            let mut out_data = vec![];
            for _ in 0..swf_tag.height {
                for _ in 0..swf_tag.width {
                    let entry = decoded_data[i] as usize;
                    if entry < palette.len() {
                        let color = &palette[entry];
                        out_data.push(color.r);
                        out_data.push(color.g);
                        out_data.push(color.b);
                        out_data.push(color.a);
                    } else {
                        out_data.push(0);
                        out_data.push(0);
                        out_data.push(0);
                        out_data.push(255);
                    }
                    i += 1;
                }
                i += (padded_width - swf_tag.width) as usize;
            }
            out_data
        }
        (2, swf::BitmapFormat::ColorMap8) => {
            let mut i = 0;
            let padded_width = (swf_tag.width + 0b11) & !0b11;

            let mut palette = Vec::with_capacity(swf_tag.num_colors as usize + 1);
            for _ in 0..=swf_tag.num_colors {
                palette.push(Color {
                    r: decoded_data[i],
                    g: decoded_data[i + 1],
                    b: decoded_data[i + 2],
                    a: decoded_data[i + 3],
                });
                i += 4;
            }
            let mut out_data = vec![];
            for _ in 0..swf_tag.height {
                for _ in 0..swf_tag.width {
                    let entry = decoded_data[i] as usize;
                    if entry < palette.len() {
                        let color = &palette[entry];
                        out_data.push(color.r);
                        out_data.push(color.g);
                        out_data.push(color.b);
                        out_data.push(color.a);
                    } else {
                        out_data.push(0);
                        out_data.push(0);
                        out_data.push(0);
                        out_data.push(0);
                    }
                    i += 1;
                }
                i += (padded_width - swf_tag.width) as usize;
            }
            out_data
        }
        _ => {
            return Err(format!(
                "Unexpected DefineBitsLossless{} format: {:?} ",
                swf_tag.version, swf_tag.format,
            )
            .into());
        }
    };

    Ok(Bitmap {
        width: swf_tag.width.into(),
        height: swf_tag.height.into(),
        data: BitmapFormat::Rgba(out_data),
    })
}

/// Decodes the bitmap data in DefineBitsLossless tag into RGBA.
/// DefineBitsLossless is Zlib encoded pixel data (similar to PNG), possibly
/// palletized.
pub fn decode_png(data: &[u8]) -> Result<Bitmap, Error> {
    use png::{ColorType, Transformations};

    let mut decoder = png::Decoder::new(data);
    // EXPAND expands palettized types to RGB.
    decoder.set_transformations(Transformations::EXPAND);
    let (info, mut reader) = decoder.read_info()?;

    let mut data = vec![0; info.buffer_size()];
    reader.next_frame(&mut data)?;

    Ok(Bitmap {
        width: info.width,
        height: info.height,
        data: if info.color_type == ColorType::RGBA {
            BitmapFormat::Rgba(data)
        } else {
            // EXPAND expands other types to RGB.
            BitmapFormat::Rgb(data)
        },
    })
}

/// Decodes the bitmap data in DefineBitsLossless tag into RGBA.
/// DefineBitsLossless is Zlib encoded pixel data (similar to PNG), possibly
/// palletized.
pub fn decode_gif(data: &[u8]) -> Result<Bitmap, Error> {
    let mut decode_options = gif::DecodeOptions::new();
    decode_options.set_color_output(gif::ColorOutput::RGBA);
    let mut reader = decode_options.read_info(data)?;
    let frame = reader.read_next_frame()?.ok_or("No frames in GIF")?;

    Ok(Bitmap {
        width: frame.width.into(),
        height: frame.height.into(),
        data: BitmapFormat::Rgba(frame.buffer.to_vec()),
    })
}

/// Images in SWFs are stored with premultiplied alpha.
/// Converts RGBA premultiplied alpha to standard RBGA.
pub fn unmultiply_alpha_rgba(rgba: &mut [u8]) {
    rgba.chunks_exact_mut(4).for_each(|rgba| {
        if rgba[3] > 0 {
            let a = f32::from(rgba[3]) / 255.0;
            rgba[0] = f32::min(f32::from(rgba[0]) / a, 255.0) as u8;
            rgba[1] = f32::min(f32::from(rgba[1]) / a, 255.0) as u8;
            rgba[2] = f32::min(f32::from(rgba[2]) / a, 255.0) as u8;
        }
    })
}

/// Converts an RGBA color from sRGB space to linear color space.
pub fn srgb_to_linear(color: [f32; 4]) -> [f32; 4] {
    fn to_linear_channel(n: f32) -> f32 {
        if n <= 0.04045 {
            n / 12.92
        } else {
            f32::powf((n + 0.055) / 1.055, 2.4)
        }
    }
    [
        to_linear_channel(color[0]),
        to_linear_channel(color[1]),
        to_linear_channel(color[2]),
        color[3],
    ]
}

/// Decodes zlib-compressed data.
fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut out_data = Vec::new();
    let mut decoder = flate2::bufread::ZlibDecoder::new(data);
    decoder.read_to_end(&mut out_data)?;
    out_data.shrink_to_fit();
    Ok(out_data)
}
