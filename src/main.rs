extern crate num;
extern crate image;
extern crate crossbeam;

use num::Complex;
use std::str::FromStr;
use image::ColorType;
use image::png::PNGEncoder;
use std::fs::File;
use std::io::Write;
use std::io::Result;

fn escape_time(c :Complex<f64>, limit: u32) -> Option<u32> {
    let mut z = Complex { re: 0.0, im: 0.0 };

    for i in 0..limit {
        z = z * z + c;
        if z.norm_sqr() > 4.0 {
            return Some(i)
        }
    }
    None
}

// コマンドライン引数のパーサー
fn parse_paier<T: FromStr>(s: &str, separator: char) -> Option<(T, T)> {
    match s.find(separator) {
        None => None,
        Some(index) => {
            // 二重マッチう〜んという感じ
            match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
                (Ok(l), Ok(r)) => Some((l, r)),
                _ => None
            }
        }
    }
}

// カンマ区切りのペアをパースして複素数を返す
fn parse_to_complex(s: &str) -> Option<Complex<f64>> {
    match parse_paier(s, ',') {
        Some((re, im)) => Some(Complex { re, im }),
        None => None
    }
}

fn pixel_to_point(bounds: (usize, usize),
                  pixel: (usize, usize),
                  uper_left: Complex<f64>,
                  lower_right: Complex<f64>)
    -> Complex<f64>
{
    let (width, height) = (
        lower_right.re - uper_left.re,
        uper_left.im - lower_right.im
    );

    Complex {
        re: uper_left.re + pixel.0 as f64 * width / bounds.0 as f64,
        im: uper_left.im - pixel.1 as f64 * height / bounds.1 as f64
    }
}

fn render(pixels: &mut [u8],
          bounds: (usize, usize),
          uper_left: Complex<f64>,
          lower_right: Complex<f64>
)
{
    assert!(pixels.len() == bounds.0 * bounds.1);

    for row in 0..bounds.1 {
        for column in 0..bounds.0 {
            let point = pixel_to_point(bounds,
                                       (column, row),
                                       uper_left, lower_right
            );

            pixels[row *bounds.0 + column] =
                match escape_time(point, 255) {
                    None => 0,
                    Some(count) => 255 - count as u8
                };
        }
    }
}

fn write_image(filename: &str, pixels: &[u8], bounds: (usize, usize)) -> Result<()>
{
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(&pixels, bounds.0 as u32, bounds.1 as u32, ColorType::Gray(8))?;

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 5 {
        writeln!(std::io::stderr(), "Usage: mandelbrot FILE PIXELS UPPERLEFT LOWERRIGHT").unwrap();
        std::process::exit(1);
    }

    let bounds = parse_paier(&args[2], 'x')
        .expect("error parsing image dimensions");
    let uper_left = parse_to_complex(&args[3])
        .expect("error parsing upper left conrner point");
    let lower_right = parse_to_complex(&args[4])
        .expect("error parsing lower right conrner point");

    let mut pixels = vec![0; bounds.0 * bounds.1];

    let threads = 8;
    let rows_per_band = bounds.1 / threads + 1;

    {
        let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * bounds.0).collect();

        crossbeam::scope(|spawner| {
            for (i, band) in bands.into_iter().enumerate() {
                let top = rows_per_band * i;
                let height = band.len() / bounds.0;
                let band_bounds = (bounds.0, height);
                let band_upper_left =
                    pixel_to_point(bounds, (0, top), uper_left, lower_right);

                let band_lower_right =
                    pixel_to_point(bounds, (bounds.0, top + height), uper_left, lower_right);

                spawner.spawn(move || {
                    render(band, band_bounds, band_upper_left, band_lower_right);
                });
            }
        });
    }

    write_image(&args[1], &pixels, bounds)
        .expect("error writing PNG file");
}
