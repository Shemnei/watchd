use colored::Colorize;
use image::DynamicImage;

use crate::pallete::rgb;

mod pallete {
	use image::Rgb;

	const PALETTE: &[char] =
		&[' ', '.', ':', ';', '+', 'o', 'O', '&', '@', '#'];

	pub(crate) fn rgb(Rgb([r, g, b]): &Rgb<u8>) -> (u8, u8, u8) {
		(*r, *g, *b)
	}

	pub(crate) fn luminance(Rgb([r, g, b]): &Rgb<u8>) -> u8 {
		let (r, g, b) = (*r as u16, *g as u16, *b as u16);

		(((r << 1) + r + (g << 2) + b) >> 3) as u8
	}

	pub(crate) fn get_char(luminance: u8) -> char {
		let idx = luminance as f32 / (u8::MAX as f32 / PALETTE.len() as f32);
		let idx = idx as usize;

		PALETTE[idx.clamp(0, PALETTE.len() - 1)]
	}
}

#[derive(Debug)]
pub struct Options {
	width: u32,
	height: u32,
}

pub struct Pixel {
	c: char,
	b: Option<(u8, u8, u8)>,
	f: Option<(u8, u8, u8)>,
}

pub struct TextImage {
	width: u32,
	height: u32,
	pixels: Vec<Pixel>,
}

impl TextImage {
	pub fn from_image(image: DynamicImage, opts: Options) -> Self {
		use crate::pallete::{get_char, luminance};

		let Options { width, height } = opts;

		let image = image.thumbnail_exact(width, height);
		let image = image.into_rgb8();

		let mut pixels: Vec<Pixel> =
			Vec::with_capacity(width as usize * height as usize);

		for y in 0..height {
			for x in 0..width {
				let p = image.get_pixel(x, y);
				let color = rgb(p);
				let b = Some(color);
				let f = None;

				let c = if f.is_some() {
					let l = luminance(p);
					get_char(l)
				} else {
					' '
				};

				pixels.push(Pixel { c, b, f });
			}
		}

		Self { width, height, pixels }
	}

	pub fn to_buffer(&self, buffer: &mut String) {
		for y in 0..self.height {
			for x in 0..self.width {
				let idx = y as usize * self.width as usize + x as usize;
				let Pixel { c, b, f } = self.pixels[idx];

				match (f, b) {
					(Some((fr, fg, fb)), Some((br, bg, bb))) => {
						let s = format!("{}", c);

						buffer.push_str(&format!(
							"{}",
							s.truecolor(fr, fg, fb).on_truecolor(br, bg, bb)
						));
					}
					(Some((r, g, b)), None) => {
						let s = format!("{}", c);

						buffer.push_str(&format!("{}", s.truecolor(r, g, b)));
					}
					(None, Some((r, g, b))) => {
						let s = format!("{}", c);

						buffer
							.push_str(&format!("{}", s.on_truecolor(r, g, b)));
					}
					_ => {
						buffer.push(c);
					}
				};
			}

			buffer.push('\r');
			buffer.push('\n');
		}
	}
}

#[test]
fn img() {
	use std::io::Write as _;

	use terminal_size::{terminal_size, Height, Width};
	let (Width(w), Height(h)) = terminal_size().unwrap();
	let (width, height) = (w as u32 / 2, h as u32);

	println!("{:?}", (width, height));

	let img = image::open("assets/tux.png").unwrap();
	let img = TextImage::from_image(img, Options { width, height });

	let mut buf = String::new();
	img.to_buffer(&mut buf);

	let mut stdout = std::io::stdout();
	stdout.write(buf.as_bytes()).unwrap();
	stdout.flush().unwrap();
}
