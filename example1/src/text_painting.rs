use gl::types::GLint;
use gl_thin::gl_fancy::GPUState;
use gl_thin::gl_helper::{GLErrorWrapper, Texture};
use rusttype::{point, Font, PositionedGlyph, Scale};

pub fn text_to_greyscale_texture(
    width: GLint,
    height: GLint,
    font_size: f32,
    message: &str,
    gpu_state: &mut GPUState,
) -> Result<Texture, GLErrorWrapper> {
    let font = Font::try_from_bytes(include_bytes!("Montserrat-Regular.ttf"))
        .expect("failed to parse font");

    let scale = Scale {
        x: font_size,
        y: font_size,
    };

    let offset = point(0.0, font.v_metrics(scale).ascent);

    let glyphs: Vec<_> = font.layout(message, scale, offset).collect();

    if true {
        let width = glyphs
            .iter()
            .rev()
            .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
            .next()
            .unwrap_or(0.0)
            .ceil() as usize;

        println!("width: {}, height: {}", width, font_size);
    }

    // let (width, height) = target.get_dimensions()?;
    let target = Texture::new()?;

    if false {
        // this doesn't work on the oculus
        let mut pixel_data = vec![99u8; (width * height) as usize];
        render_glyphs_to_grey(width, height, &glyphs, &mut pixel_data);
        target
            .bound(gl::TEXTURE_2D, gpu_state)?
            .write_pixels_and_generate_mipmap(
                // gl::TEXTURE_2D,
                0,
                gl::RGB as GLint,
                width,
                height,
                gl::RED,
                pixel_data.as_slice(),
            )?;
    } else {
        let mut pixel_data = vec![0u8; (3 * width * height) as usize];
        render_glyphs_to_rgb(width, height, &glyphs, &mut pixel_data);

        if true {
            log::debug!(
                "text pixels {:?} .. {:?}",
                pixel_data.iter().min(),
                pixel_data.iter().max()
            );
        }

        target
            .bound(gl::TEXTURE_2D, gpu_state)?
            .write_pixels_and_generate_mipmap(
                0,
                gl::RGB as GLint,
                width,
                height,
                gl::RGB,
                pixel_data.as_slice(),
            )?;
    }
    Ok(target)
}

pub fn render_glyphs_to_grey<'a, 'f: 'a>(
    width: i32,
    height: i32,
    glyphs: impl IntoIterator<Item = &'a PositionedGlyph<'f>>,
    pixel_data: &mut [u8],
) {
    for g in glyphs {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|x0, y0, v| {
                let x = x0 as i32 + bb.min.x;
                let y = y0 as i32 + bb.min.y;
                if x >= 0 && x < width && y >= 0 && y < height {
                    let idx = x + y * width;
                    pixel_data[idx as usize] = ((1.0 - v) * 255.9) as u8;
                }
            })
        }
    }
}

pub fn render_glyphs_to_rgb<'a, 'f: 'a>(
    width: i32,
    height: i32,
    glyphs: impl IntoIterator<Item = &'a PositionedGlyph<'f>>,
    pixel_data: &mut [u8],
) {
    for g in glyphs {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|x0, y0, v| {
                let x = x0 as i32 + bb.min.x;
                let y = y0 as i32 + bb.min.y;
                if x >= 0 && x < width && y >= 0 && y < height {
                    let idx = (3 * (x + y * width)) as usize;
                    let a = (v * 255.9) as u8;
                    pixel_data[idx] = a;
                    pixel_data[idx + 1] = a;
                    pixel_data[idx + 2] = a;
                }
            })
        }
    }
}
