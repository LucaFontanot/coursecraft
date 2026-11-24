use std::io::Cursor;

use image::codecs::gif::{GifEncoder, Repeat};
use image::codecs::webp::WebPEncoder;
use image::{Delay, ExtendedColorType, Frame, ImageBuffer, ImageFormat, Rgb, Rgba};

use crate::animation::{MAX_ANIMATION_FRAMES, clamp_frame_delay_ms, decode_animation};
use crate::svg::{MAX_SVG_BYTES, decode_svg, svg_natural_size};
use crate::{
    FontFileStyle, Limits, MAX_FONT_BYTES, MediaError, decode_image, decode_image_fitted,
    encode_png_rgba, font_info,
};

use crate::text_metrics::{INTER_BOLD, INTER_ITALIC, INTER_REGULAR};

fn make_png_2x2() -> Vec<u8> {
    let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(2, 2);
    img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
    img.put_pixel(1, 0, Rgba([0, 255, 0, 255]));
    img.put_pixel(0, 1, Rgba([0, 0, 255, 255]));
    img.put_pixel(1, 1, Rgba([255, 255, 255, 128]));

    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .expect("PNG encoding failed");
    bytes
}

fn make_jpeg_2x2() -> Vec<u8> {
    let mut img = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(2, 2);
    img.put_pixel(0, 0, Rgb([200, 50, 50]));
    img.put_pixel(1, 0, Rgb([50, 200, 50]));
    img.put_pixel(0, 1, Rgb([50, 50, 200]));
    img.put_pixel(1, 1, Rgb([200, 200, 200]));

    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Jpeg)
        .expect("JPEG encoding failed");
    bytes
}

fn make_animated_gif_2x2() -> Vec<u8> {
    let mut frame1 = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(2, 2);
    for pixel in frame1.pixels_mut() {
        *pixel = Rgba([255, 0, 0, 255]); // Red
    }

    let mut frame2 = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(2, 2);
    for pixel in frame2.pixels_mut() {
        *pixel = Rgba([0, 0, 255, 255]); // Blue
    }

    let mut bytes = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut bytes);
        encoder
            .set_repeat(Repeat::Infinite)
            .expect("set repeat failed");
        encoder
            .encode_frame(Frame::from_parts(
                frame1,
                0,
                0,
                Delay::from_numer_denom_ms(100, 1),
            ))
            .expect("encode frame 1 failed");
        encoder
            .encode_frame(Frame::from_parts(
                frame2,
                0,
                0,
                Delay::from_numer_denom_ms(100, 1),
            ))
            .expect("encode frame 2 failed");
    }
    bytes
}

#[test]
fn decodes_2x2_png() {
    let bytes = make_png_2x2();
    let img = decode_image(&bytes, &Limits::default()).expect("failed to decode PNG");
    assert_eq!(img.width, 2);
    assert_eq!(img.height, 2);
    assert_eq!(img.pixels.len(), 16);
    // pixel (0, 0)
    assert_eq!(&img.pixels[0..4], &[255, 0, 0, 255]);
    // pixel (1, 0)
    assert_eq!(&img.pixels[4..8], &[0, 255, 0, 255]);
    // pixel (0, 1)
    assert_eq!(&img.pixels[8..12], &[0, 0, 255, 255]);
    // pixel (1, 1)
    assert_eq!(&img.pixels[12..16], &[255, 255, 255, 128]);
}

#[test]
fn decodes_2x2_jpeg() {
    let bytes = make_jpeg_2x2();
    let img = decode_image(&bytes, &Limits::default()).expect("failed to decode JPEG");
    assert_eq!(img.width, 2);
    assert_eq!(img.height, 2);
    assert_eq!(img.pixels.len(), 16);
    // JPEG converts to RGBA with alpha = 255
    assert_eq!(img.pixels[3], 255);
    assert_eq!(img.pixels[7], 255);
    assert_eq!(img.pixels[11], 255);
    assert_eq!(img.pixels[15], 255);
}

#[test]
fn decodes_first_frame_of_gif() {
    let bytes = make_animated_gif_2x2();
    let img = decode_image(&bytes, &Limits::default()).expect("failed to decode GIF");
    assert_eq!(img.width, 2);
    assert_eq!(img.height, 2);
    assert_eq!(img.pixels.len(), 16);
    // First frame is solid red
    for chunk in img.pixels.as_chunks::<4>().0 {
        assert_eq!(chunk, &[255, 0, 0, 255]);
    }
}

#[test]
fn rejects_empty_input() {
    let err = decode_image(&[], &Limits::default()).expect_err("empty should fail");
    assert_eq!(err, MediaError::Unsupported);
}

#[test]
fn rejects_random_bytes() {
    let random_data = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
    let err = decode_image(&random_data, &Limits::default()).expect_err("random should fail");
    assert_eq!(err, MediaError::Unsupported);
}

#[test]
fn rejects_truncated_bytes() {
    let png = make_png_2x2();
    assert!(png.len() > 10);
    // Truncate PNG to first 12 bytes (signature is present, but IHDR is truncated)
    let truncated = &png[..12];
    let err = decode_image(truncated, &Limits::default()).expect_err("truncated should fail");
    assert!(matches!(err, MediaError::Corrupt(_)));
}

#[test]
fn rejects_image_over_width_limit() {
    let png = make_png_2x2();
    let limits = Limits {
        max_width: 1,
        max_height: 10,
        max_decoded_bytes: 1024,
    };
    let err = decode_image(&png, &limits).expect_err("over width limit should fail");
    assert_eq!(
        err,
        MediaError::TooLarge {
            width: 2,
            height: 2
        }
    );
}

#[test]
fn rejects_image_over_height_limit() {
    let png = make_png_2x2();
    let limits = Limits {
        max_width: 10,
        max_height: 1,
        max_decoded_bytes: 1024,
    };
    let err = decode_image(&png, &limits).expect_err("over height limit should fail");
    assert_eq!(
        err,
        MediaError::TooLarge {
            width: 2,
            height: 2
        }
    );
}

#[test]
fn rejects_image_over_bytes_limit() {
    let png = make_png_2x2();
    // 2x2 RGBA is 16 bytes. Cap at 15.
    let limits = Limits {
        max_width: 10,
        max_height: 10,
        max_decoded_bytes: 15,
    };
    let err = decode_image(&png, &limits).expect_err("over bytes limit should fail");
    assert_eq!(
        err,
        MediaError::TooLarge {
            width: 2,
            height: 2
        }
    );
}

/// U-01: `decode_image_fitted` downscales oversized rasters while preserving
/// exact natural dimensions, skips downscaling for already-small images, and
/// rejects hostile/empty/oversized inputs with `Err`.
#[test]
fn u01_decode_image_fitted_downscales_and_preserves_natural_dims() {
    let pixels = vec![42u8; 1280 * 720 * 4];
    let large_png = encode_png_rgba(1280, 720, &pixels).expect("encode 1280x720 png");

    // Downscale to <= 384 px preserving 16:9 aspect ratio and natural [1280, 720].
    let (fitted, natural) = decode_image_fitted(&large_png, Some((384, 384)), &Limits::default())
        .expect("decode fitted 1280x720");
    assert_eq!(natural, [1280, 720]);
    assert!(fitted.width >= 1 && fitted.width <= 384);
    assert!(fitted.height >= 1 && fitted.height <= 384);
    assert_eq!((fitted.width, fitted.height), (384, 216));
    assert_eq!(
        fitted.pixels.len(),
        (fitted.width as usize) * (fitted.height as usize) * 4
    );

    // None max_dim returns full resolution.
    let (full, full_natural) =
        decode_image_fitted(&large_png, None, &Limits::default()).expect("decode full 1280x720");
    assert_eq!(full_natural, [1280, 720]);
    assert_eq!((full.width, full.height), (1280, 720));

    // Already small image skips downscaling.
    let small_png = make_png_2x2();
    let (small, small_natural) =
        decode_image_fitted(&small_png, Some((384, 384)), &Limits::default())
            .expect("decode small png");
    assert_eq!(small_natural, [2, 2]);
    assert_eq!((small.width, small.height), (2, 2));

    // Zero max_dim is treated safely without panicking.
    let (zero_guard, zero_natural) =
        decode_image_fitted(&small_png, Some((0, 0)), &Limits::default())
            .expect("zero max_dim should not panic");
    assert_eq!(zero_natural, [2, 2]);
    assert_eq!((zero_guard.width, zero_guard.height), (2, 2));

    // Hostile, empty, truncated, and over-limit inputs return Err.
    assert_eq!(
        decode_image_fitted(&[], Some((384, 384)), &Limits::default()),
        Err(MediaError::Unsupported)
    );
    assert_eq!(
        decode_image_fitted(
            &[1, 2, 3, 4, 5, 6, 7, 8],
            Some((384, 384)),
            &Limits::default()
        ),
        Err(MediaError::Unsupported)
    );
    assert!(matches!(
        decode_image_fitted(&large_png[..12], Some((384, 384)), &Limits::default()),
        Err(MediaError::Corrupt(_))
    ));
    let tight = Limits {
        max_width: 640,
        max_height: 640,
        max_decoded_bytes: 64 * 1024 * 1024,
    };
    assert_eq!(
        decode_image_fitted(&large_png, Some((384, 384)), &tight),
        Err(MediaError::TooLarge {
            width: 1280,
            height: 720
        })
    );
}

/// Build a tiny but valid single-font sfnt with a `name` table and, optionally,
/// an OS/2 table. `fs_type == None` omits OS/2 entirely; `licence == None`
/// omits the name ID 13 record.
fn synthetic_font(fs_type: Option<u16>, licence: Option<&str>) -> Vec<u8> {
    fn u16be(out: &mut Vec<u8>, value: u16) {
        out.extend_from_slice(&value.to_be_bytes());
    }
    fn u32be(out: &mut Vec<u8>, value: u32) {
        out.extend_from_slice(&value.to_be_bytes());
    }

    let family: Vec<u16> = "Test Family".encode_utf16().collect();
    let licence_units: Vec<u16> = licence.unwrap_or_default().encode_utf16().collect();

    let has_os2 = fs_type.is_some();
    let num_tables = u16::from(has_os2) + 1;
    let os2_offset = 12u32 + 16 * u32::from(num_tables);
    let os2_len = 78u32;
    let name_offset = if has_os2 {
        os2_offset + os2_len
    } else {
        os2_offset
    };

    let num_records = u16::from(licence.is_some()) + 1;
    let string_offset = 6u16 + 12 * num_records;
    let name_len =
        u32::from(string_offset) + 2 * family.len() as u32 + 2 * licence_units.len() as u32;

    let mut out = Vec::new();
    u32be(&mut out, 0x0001_0000); // sfntVersion: TrueType
    u16be(&mut out, num_tables);
    u16be(&mut out, 0); // searchRange
    u16be(&mut out, 0); // entrySelector
    u16be(&mut out, 0); // rangeShift

    if has_os2 {
        u32be(&mut out, 0x4F53_2F32); // 'OS/2'
        u32be(&mut out, 0); // checksum
        u32be(&mut out, os2_offset);
        u32be(&mut out, os2_len);
    }
    u32be(&mut out, 0x6E61_6D65); // 'name'
    u32be(&mut out, 0);
    u32be(&mut out, name_offset);
    u32be(&mut out, name_len);

    if let Some(fs) = fs_type {
        debug_assert_eq!(out.len() as u32, os2_offset);
        u16be(&mut out, 0); // version
        u16be(&mut out, 0); // xAvgCharWidth
        u16be(&mut out, 400); // usWeightClass
        u16be(&mut out, 5); // usWidthClass
        u16be(&mut out, fs); // fsType
        out.resize(os2_offset as usize + os2_len as usize, 0);
    }

    debug_assert_eq!(out.len() as u32, name_offset);
    u16be(&mut out, 0); // format
    u16be(&mut out, num_records);
    u16be(&mut out, string_offset);

    // Family name (name ID 1), en-US.
    u16be(&mut out, 3); // platformID: Windows
    u16be(&mut out, 1); // encodingID: Unicode BMP
    u16be(&mut out, 0x0409); // languageID: en-US
    u16be(&mut out, 1);
    u16be(&mut out, (2 * family.len()) as u16);
    u16be(&mut out, 0);
    if licence.is_some() {
        // Licence text (name ID 13), en-US.
        u16be(&mut out, 3);
        u16be(&mut out, 1);
        u16be(&mut out, 0x0409);
        u16be(&mut out, 13);
        u16be(&mut out, (2 * licence_units.len()) as u16);
        u16be(&mut out, (2 * family.len()) as u16);
    }
    for unit in family {
        u16be(&mut out, unit);
    }
    for unit in licence_units {
        u16be(&mut out, unit);
    }
    out
}

#[test]
fn font_info_reads_family_and_style_from_the_name_table() {
    let regular = font_info(INTER_REGULAR).expect("read Inter Regular");
    assert_eq!(regular.family, "Inter");
    assert_eq!(regular.style, FontFileStyle::Regular);

    let bold = font_info(INTER_BOLD).expect("read Inter Bold");
    assert_eq!(bold.family, "Inter");
    assert_eq!(bold.style, FontFileStyle::Bold);

    let italic = font_info(INTER_ITALIC).expect("read Inter Italic");
    assert_eq!(italic.family, "Inter");
    assert_eq!(italic.style, FontFileStyle::Italic);
}

#[test]
fn font_info_reads_embedding_restriction() {
    let restricted = font_info(&synthetic_font(Some(0x0002), None)).expect("read restricted font");
    assert!(restricted.embedding_restricted);

    // Other fsType bits do not count as restricted embedding.
    let unrestricted = font_info(&synthetic_font(Some(0x0004), None)).expect("read open font");
    assert!(!unrestricted.embedding_restricted);

    // No OS/2 table means no restriction.
    let no_os2 = font_info(&synthetic_font(None, None)).expect("read font without OS/2");
    assert!(!no_os2.embedding_restricted);
}

#[test]
fn font_info_reads_and_cleans_licence() {
    let with_licence = font_info(&synthetic_font(
        Some(0),
        Some("  Licensed \u{0007} for testing.  "),
    ))
    .expect("read font with licence");
    assert_eq!(
        with_licence.licence.as_deref(),
        Some("Licensed  for testing.")
    );

    let no_licence = font_info(&synthetic_font(Some(0), None)).expect("read font without licence");
    assert_eq!(no_licence.licence, None);

    let empty = font_info(&synthetic_font(Some(0), Some("   "))).expect("read blank licence");
    assert_eq!(empty.licence, None);

    let long = "x".repeat(250);
    let capped = font_info(&synthetic_font(Some(0), Some(&long))).expect("read long licence");
    assert_eq!(capped.licence.as_deref().map(str::len), Some(200));
}

#[test]
fn font_info_rejects_bad_and_oversized_files() {
    assert!(matches!(
        font_info(b"this is not a font"),
        Err(MediaError::InvalidFont(_))
    ));
    assert!(matches!(font_info(&[]), Err(MediaError::InvalidFont(_))));
    let oversized = vec![0u8; MAX_FONT_BYTES.saturating_add(1)];
    assert!(matches!(
        font_info(&oversized),
        Err(MediaError::FontTooLarge { .. })
    ));
}

#[test]
fn encodes_and_decodes_2x2_png() {
    let pixels: Vec<u8> = vec![
        255, 0, 0, 255, // red
        0, 255, 0, 255, // green
        0, 0, 255, 255, // blue
        255, 255, 0, 128, // yellow semi-transparent
    ];
    let encoded = encode_png_rgba(2, 2, &pixels).expect("encode 2x2 png");
    assert!(encoded.starts_with(b"\x89PNG\r\n\x1a\n"));

    let decoded = decode_image(&encoded, &Limits::default()).expect("decode encoded png");
    assert_eq!(decoded.width, 2);
    assert_eq!(decoded.height, 2);
    assert_eq!(decoded.pixels, pixels);
}

#[test]
fn encode_png_rejects_zero_or_mismatched_dimensions() {
    assert!(encode_png_rgba(0, 10, &[]).is_err());
    assert!(encode_png_rgba(10, 0, &[]).is_err());
    assert!(encode_png_rgba(2, 2, &[0, 0, 0]).is_err());
}

// ---------------------------------------------------------------- SVG tests

fn simple_svg(width: u32, height: u32) -> Vec<u8> {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><rect width="{width}" height="{height}" fill="#ff0000"/></svg>"##
    )
    .into_bytes()
}

#[test]
fn svg_decodes_at_two_sizes_with_pixels() {
    let bytes = simple_svg(40, 20);
    for (w, h) in [(8u32, 4u32), (40, 20)] {
        let image = decode_svg(&bytes, (w, h), &Limits::default()).expect("decode SVG");
        assert_eq!((image.width, image.height), (w, h));
        assert_eq!(image.pixels.len(), (w * h * 4) as usize);
        for pixel in image.pixels.as_chunks::<4>().0 {
            assert_eq!(pixel, &[255, 0, 0, 255]);
        }
    }
}

#[test]
fn svg_natural_size_reads_width_and_height() {
    assert_eq!(svg_natural_size(&simple_svg(40, 20)), Some((40.0, 20.0)));
}

#[test]
fn svg_external_reference_is_blocked() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="4" height="4" viewBox="0 0 4 4">
        <rect width="4" height="4" fill="#00ff00"/>
        <image xlink:href="file:///etc/passwd" width="4" height="4"/>
        <image href="https://example.com/picture.png" width="4" height="4"/>
    </svg>"##;
    // The rectangle still renders; the external images are dropped, never read.
    let image = decode_svg(svg, (4, 4), &Limits::default()).expect("decode SVG");
    for pixel in image.pixels.as_chunks::<4>().0 {
        assert_eq!(pixel, &[0, 255, 0, 255]);
    }
}

#[test]
fn svg_rejects_garbage_and_oversized_input_without_panicking() {
    assert!(matches!(
        decode_svg(b"not an svg at all", (4, 4), &Limits::default()),
        Err(MediaError::Corrupt(_))
    ));
    let oversized = vec![b' '; MAX_SVG_BYTES.saturating_add(1)];
    assert!(matches!(
        decode_svg(&oversized, (4, 4), &Limits::default()),
        Err(MediaError::SvgTooLarge { .. })
    ));
    assert_eq!(svg_natural_size(b"not an svg"), None);
}

#[test]
fn svg_output_is_clamped_to_the_pixel_cap() {
    let limits = Limits {
        max_width: 10,
        max_height: 10,
        max_decoded_bytes: 100,
    };
    let image = decode_svg(&simple_svg(40, 20), (1000, 1000), &limits).expect("decode SVG");
    assert!(image.width <= 10 && image.height <= 10);
    assert!(u64::from(image.width) * u64::from(image.height) * 4 <= 100);
}

// --------------------------------------------------------- animation tests

fn make_gif(frames: &[Rgba<u8>], delays_ms: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut bytes);
        encoder
            .set_repeat(Repeat::Infinite)
            .expect("set repeat failed");
        for (colour, delay) in frames.iter().zip(delays_ms) {
            let mut buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(2, 2);
            for pixel in buffer.pixels_mut() {
                *pixel = *colour;
            }
            encoder
                .encode_frame(Frame::from_parts(
                    buffer,
                    0,
                    0,
                    Delay::from_numer_denom_ms(*delay, 1),
                ))
                .expect("encode frame failed");
        }
    }
    bytes
}

fn put_u24(out: &mut Vec<u8>, value: u32) {
    let value = value & 0x00FF_FFFF;
    out.extend_from_slice(&value.to_le_bytes()[..3]);
}

fn riff_chunk(out: &mut Vec<u8>, fourcc: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(fourcc);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
}

fn still_webp_lossless(pixels: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    WebPEncoder::new_lossless(&mut out)
        .encode(pixels, width, height, ExtendedColorType::Rgba8)
        .expect("encode still WebP");
    out
}

/// Build a minimal animated WebP around still lossless VP8L frames using
/// `image`'s encoder. ANMF/ANIM/VP8X follow the WebP container format.
fn make_animated_webp_2x2(delays_ms: &[u32]) -> Vec<u8> {
    let width = 2u32;
    let height = 2u32;
    let mut frames = Vec::new();
    for (index, delay) in delays_ms.iter().enumerate() {
        let colour = if index % 2 == 0 { 200 } else { 40 };
        let pixels: Vec<u8> = std::iter::repeat_n([colour, 40, 200, 255], 4)
            .flatten()
            .collect();
        let still = still_webp_lossless(&pixels, width, height);
        let size = u32::from_le_bytes([still[16], still[17], still[18], still[19]]) as usize;
        let vp8l = still.get(20..20 + size).expect("VP8L payload");

        let mut anmf = Vec::new();
        put_u24(&mut anmf, 0); // x / 2
        put_u24(&mut anmf, 0); // y / 2
        put_u24(&mut anmf, width - 1);
        put_u24(&mut anmf, height - 1);
        put_u24(&mut anmf, *delay);
        anmf.push(0x02); // do not blend: replace the canvas
        riff_chunk(&mut anmf, b"VP8L", vp8l);
        frames.push(anmf);
    }

    let mut chunks = Vec::new();
    let mut vp8x = vec![0x02 | 0x10, 0, 0, 0];
    put_u24(&mut vp8x, width - 1);
    put_u24(&mut vp8x, height - 1);
    riff_chunk(&mut chunks, b"VP8X", &vp8x);

    let mut anim = vec![0, 0, 0, 0];
    anim.extend_from_slice(&0u16.to_le_bytes()); // loop forever
    riff_chunk(&mut chunks, b"ANIM", &anim);
    for frame in &frames {
        riff_chunk(&mut chunks, b"ANMF", frame);
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((chunks.len() + 4) as u32).to_le_bytes());
    out.extend_from_slice(b"WEBP");
    out.extend_from_slice(&chunks);
    out
}

#[test]
fn gif_animation_decodes_frames_and_delays() {
    let bytes = make_gif(
        &[Rgba([255, 0, 0, 255]), Rgba([0, 0, 255, 255])],
        &[100, 40],
    );
    let animation = decode_animation(&bytes, &Limits::default())
        .expect("decode GIF")
        .expect("animated GIF");
    assert_eq!(animation.frames.len(), 2);
    assert_eq!(animation.frames[0].delay_ms, 100);
    assert_eq!(animation.frames[1].delay_ms, 40);
    assert_eq!((animation.width, animation.height), (2, 2));
    assert_eq!(animation.frame_index_at(0), 0);
    assert_eq!(animation.frame_index_at(99), 0);
    assert_eq!(animation.frame_index_at(100), 1);
    assert_eq!(animation.frame_index_at(140), 0); // loop
}

#[test]
fn tiny_and_zero_delays_become_one_hundred_ms() {
    let bytes = make_gif(&[Rgba([1, 2, 3, 255]), Rgba([4, 5, 6, 255])], &[0, 10]);
    let animation = decode_animation(&bytes, &Limits::default())
        .expect("decode GIF")
        .expect("animated GIF");
    assert_eq!(animation.frames[0].delay_ms, 100);
    assert_eq!(animation.frames[1].delay_ms, 100);
    assert_eq!(clamp_frame_delay_ms(0), 100);
    assert_eq!(clamp_frame_delay_ms(10), 100);
    assert_eq!(clamp_frame_delay_ms(15), 20);
    assert_eq!(clamp_frame_delay_ms(50_000), 10_000);
}

#[test]
fn webp_animation_decodes_frames_and_delays() {
    let bytes = make_animated_webp_2x2(&[60, 120]);
    let animation = decode_animation(&bytes, &Limits::default())
        .expect("decode WebP")
        .expect("animated WebP");
    assert_eq!(animation.frames.len(), 2);
    assert_eq!(animation.frames[0].delay_ms, 60);
    assert_eq!(animation.frames[1].delay_ms, 120);
    assert_eq!(animation.frame_index_at(59), 0);
    assert_eq!(animation.frame_index_at(60), 1);
    assert_eq!(animation.frame_index_at(180), 0);
}

#[test]
fn still_images_are_not_animations() {
    assert_eq!(
        decode_animation(&make_png_2x2(), &Limits::default()).expect("PNG is not animation"),
        None
    );
    let still = still_webp_lossless(
        &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 9, 9, 9, 255],
        2,
        2,
    );
    assert_eq!(
        decode_animation(&still, &Limits::default()).expect("still WebP is not animation"),
        None
    );
}

#[test]
fn animation_frame_cap_is_enforced() {
    let colours: Vec<Rgba<u8>> = (0..=MAX_ANIMATION_FRAMES)
        .map(|index| Rgba([index as u8, 0, 0, 255]))
        .collect();
    let delays = vec![20u32; colours.len()];
    let bytes = make_gif(&colours, &delays);
    assert!(matches!(
        decode_animation(&bytes, &Limits::default()),
        Err(MediaError::AnimationTooLarge { .. })
    ));
}

#[test]
fn animation_decoded_size_cap_is_enforced() {
    let bytes = make_gif(
        &[Rgba([255, 0, 0, 255]), Rgba([0, 0, 255, 255])],
        &[100, 100],
    );
    let limits = Limits {
        max_width: 10,
        max_height: 10,
        max_decoded_bytes: 8,
    };
    assert!(matches!(
        decode_animation(&bytes, &limits),
        Err(MediaError::AnimationTooLarge { .. }) | Err(MediaError::TooLarge { .. })
    ));
}

#[test]
fn fs_type_allows_restricted_when_permissive_bit_also_set() {
    let preview_also =
        font_info(&synthetic_font(Some(0x0006), None)).expect("restricted + preview&print");
    assert!(!preview_also.embedding_restricted);

    let editable_also =
        font_info(&synthetic_font(Some(0x000A), None)).expect("restricted + editable");
    assert!(!editable_also.embedding_restricted);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn system_font_index_handles_lookup_fallback_ttc_corrupt_and_missing_dirs() {
    use crate::system_fonts::SystemFontIndex;

    let dir = std::env::temp_dir().join("coursecraft-media-sysfont-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp font dir");

    // Empty dir -> empty index.
    assert!(
        SystemFontIndex::from_folders(std::slice::from_ref(&dir))
            .families()
            .is_empty()
    );
    // Missing dir -> empty index.
    assert!(
        SystemFontIndex::from_folders(&[dir.join("does-not-exist")])
            .families()
            .is_empty()
    );

    // Copy a real repo font under a different filename, plus a corrupt file.
    let dm_serif =
        include_bytes!("../../../examples/healthy-habits/media/fonts/dm-serif-display-regular.ttf");
    let font_path = dir.join("custom-display.ttf");
    std::fs::write(&font_path, dm_serif).expect("write dm serif");
    std::fs::write(dir.join("corrupt.ttf"), b"not a font").expect("write corrupt");

    // Build a 2-face .ttc collection from two copies of synthetic_font,
    // shifting each face's table offsets by its start offset in the collection.
    let mut f1 = synthetic_font(Some(0x0000), None);
    let mut f2 = synthetic_font(Some(0x0002), None);
    let header_len = 12u32 + 2 * 4;
    let off1 = header_len;
    let off2 = off1 + f1.len() as u32;
    let shift_offsets = |sfnt: &mut [u8], delta: u32| {
        let num_tables = u16::from_be_bytes([sfnt[4], sfnt[5]]) as usize;
        for i in 0..num_tables {
            let entry = 12 + i * 16;
            let orig = u32::from_be_bytes([
                sfnt[entry + 8],
                sfnt[entry + 9],
                sfnt[entry + 10],
                sfnt[entry + 11],
            ]);
            let shifted = (orig + delta).to_be_bytes();
            sfnt[entry + 8..entry + 12].copy_from_slice(&shifted);
        }
    };
    shift_offsets(&mut f1, off1);
    shift_offsets(&mut f2, off2);
    let mut ttc = Vec::new();
    ttc.extend_from_slice(b"ttcf");
    ttc.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    ttc.extend_from_slice(&2u32.to_be_bytes());
    ttc.extend_from_slice(&off1.to_be_bytes());
    ttc.extend_from_slice(&off2.to_be_bytes());
    ttc.extend_from_slice(&f1);
    ttc.extend_from_slice(&f2);
    std::fs::write(dir.join("pair.ttc"), &ttc).expect("write ttc");

    let index = SystemFontIndex::from_folders(std::slice::from_ref(&dir));
    assert!(index.has_family("dm serif display"));
    assert!(index.has_family("  DM Serif Display  "));
    assert!(index.has_family("test family"));
    assert_eq!(
        index.find_family("Test Family").map(|f| f.name.as_str()),
        Some("Test Family")
    );

    // Style fallback: Bold requested when only Regular exists -> resolves to Regular.
    let (fam, face) = index
        .resolve_face("DM SERIF DISPLAY", FontFileStyle::Bold)
        .expect("resolve with fallback to regular");
    assert_eq!(fam.name, "DM Serif Display");
    assert_eq!(face.style, FontFileStyle::Regular);

    // Removing the file after indexing (before loading bytes) returns None without crashing.
    let index_uncached = SystemFontIndex::from_folders(std::slice::from_ref(&dir));
    let (_, uncached_face) = index_uncached
        .resolve_face("DM Serif Display", FontFileStyle::Regular)
        .expect("resolve before delete");
    std::fs::remove_file(&font_path).expect("remove font file");
    assert!(index_uncached.load_face_bytes(uncached_face).is_none());

    // Now load from the first index? No, file was deleted; test load_face_bytes on existing ttc.
    let (_, ttc_face) = index
        .resolve_face("Test Family", FontFileStyle::Regular)
        .expect("resolve ttc face");
    assert!(index.load_face_bytes(ttc_face).is_some());

    let _ = std::fs::remove_dir_all(&dir);
}
