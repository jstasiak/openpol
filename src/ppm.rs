use std::io;

/// Write an image stored in RGB values in `data` to a writer using PPM text format. `data` is
/// interpreted as `width * height` 3-byte RGB values stored row by row.
pub fn write_ppm<T: io::Write>(
    width: usize,
    height: usize,
    data: &[u8],
    mut w: T,
) -> io::Result<()> {
    write!(w, "P3\n{} {}\n255\n", width, height)?;

    for y in 0..height {
        for x in 0..width {
            let offset = 3 * (y * width + x);
            let color = &data[offset..offset + 3];
            write!(w, "{} {} {} ", color[0], color[1], color[2])?;
        }
        writeln!(w)?;
    }
    w.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::ppm::write_ppm;
    use std::str;

    #[test]
    fn test_write_ppm_works() {
        let data = [255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255];
        let mut buffer = Vec::new();
        write_ppm(3, 2, &data, &mut buffer).unwrap();
        let got = str::from_utf8(&buffer).unwrap();
        let expected = "P3
3 2
255
255 0 0 0 0 0 0 0 0 
0 0 0 0 0 0 255 255 255 
";
        assert_eq!(got, expected);
    }
}
