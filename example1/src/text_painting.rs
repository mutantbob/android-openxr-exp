use gl::types::GLint;
use gl_thin::gl_helper::{explode_if_gl_error, GLErrorWrapper, Texture};
use rusttype::{point, Font, Scale};

struct Banana {}

pub fn banana(width: GLint, height: GLint) -> Result<Texture, GLErrorWrapper> {
    let font = Font::try_from_bytes(include_bytes!("Montserrat-Regular.ttf"))
        .expect("failed to parse font");

    let pixel_height = 40;
    let font_size = pixel_height as f32;
    let scale = Scale {
        x: font_size,
        y: font_size,
    };

    let offset = point(0.0, font.v_metrics(scale).ascent);

    let glyphs: Vec<_> = font.layout("Hail Bob!", scale, offset).collect();

    if true {
        let width = glyphs
            .iter()
            .rev()
            .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
            .next()
            .unwrap_or(0.0)
            .ceil() as usize;

        println!("width: {}, height: {}", width, pixel_height);
    }

    // let (width, height) = target.get_dimensions()?;
    let mut target = Texture::new().unwrap();

    if false {
        // this doesn't work on the oculus
        let mut pixel_data = vec![99u8; (width * height) as usize];
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
        target
            .write_pixels(
                gl::TEXTURE_2D,
                0,
                gl::RED as GLint,
                width,
                height,
                gl::RED,
                pixel_data.as_slice(),
            )
            .unwrap();
    } else {
        let mut pixel_data = vec![99u8; (3 * width * height) as usize];
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

        if true {
            log::debug!(
                "text pixels {:?} .. {:?}",
                pixel_data.iter().min(),
                pixel_data.iter().max()
            );
        }

        target
            .write_pixels(
                gl::TEXTURE_2D,
                0,
                gl::RGB as GLint,
                width,
                height,
                gl::RGB,
                pixel_data.as_slice(),
            )
            .unwrap();

        unsafe { gl::GenerateMipmap(gl::TEXTURE_2D) };
        explode_if_gl_error()?;
    }
    Ok(target)
}
