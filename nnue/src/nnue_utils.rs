use std::{fmt::Display, io::{self, Read}};

// --- LEB128 signed ---
pub fn read_leb128_i16(r: &mut impl Read, n: usize) -> io::Result<Vec<i16>> {
    let mut magic = [0u8; 17];
    r.read_exact(&mut magic)?;
    assert!(&magic == b"COMPRESSED_LEB128");
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let mut remaining = u32::from_le_bytes(len_buf);
    let mut buf = vec![];
    while remaining > 0 {
        let mut chunk = [0u8; 4096];
        let take = remaining.min(4096);
        r.read_exact(&mut chunk[..take as usize])?;
        buf.extend_from_slice(&chunk[..take as usize]);
        remaining -= take;
    }
    let mut out = Vec::with_capacity(n);
    let mut i = 0;
    for _ in 0..n {
        let mut shift = 0;
        let mut val: i32 = 0;
        loop {
            let byte = buf[i] as i32;
            i += 1;
            val |= (byte & 0x7f) << shift;
            shift += 7;
            if (byte & 0x80) == 0 {
                if shift < 32 && (byte & 0x40) != 0 {
                    val |= !0 << shift;
                }
                out.push(val as i16);
                break;
            }
        }
    }
    Ok(out)
}

pub fn read_leb128_i32(r: &mut impl Read, n: usize) -> io::Result<Vec<i32>> {
    let mut magic = [0u8; 17];
    r.read_exact(&mut magic)?;
    assert!(&magic == b"COMPRESSED_LEB128");
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let mut remaining = u32::from_le_bytes(len_buf);
    let mut buf = vec![];
    while remaining > 0 {
        let mut chunk = [0u8; 4096];
        let take = remaining.min(4096);
        r.read_exact(&mut chunk[..take as usize])?;
        buf.extend_from_slice(&chunk[..take as usize]);
        remaining -= take;
    }
    let mut out = Vec::with_capacity(n);
    let mut i = 0;
    for _ in 0..n {
        let mut shift = 0;
        let mut val: i32 = 0;
        loop {
            let byte = buf[i] as i32;
            i += 1;
            val |= (byte & 0x7f) << shift;
            shift += 7;
            if (byte & 0x80) == 0 {
                if shift < 32 && (byte & 0x40) != 0 {
                    val |= !0 << shift;
                }
                out.push(val);
                break;
            }
        }
    }
    Ok(out)
}

// --- Little endian helpers ---
pub fn read_u32(r: &mut impl Read) -> io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}
pub fn read_i8(r: &mut impl Read) -> io::Result<i8> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b)?;
    Ok(i8::from_le_bytes(b))
}
pub fn read_i32_vec(r: &mut impl Read, n: usize) -> io::Result<Vec<i32>> {
    let mut v = vec![0i32; n];
    r.read_exact(bytemuck::cast_slice_mut(&mut v))?;
    Ok(v)
}
pub fn read_i16_vec(r: &mut impl Read, n: usize) -> io::Result<Vec<i16>> {
    let mut v = vec![0i16; n];
    r.read_exact(bytemuck::cast_slice_mut(&mut v))?;
    Ok(v)
}
pub fn read_i8_vec(r: &mut impl Read, n: usize) -> io::Result<Vec<i8>> {
    let mut v = vec![0i8; n];
    r.read_exact(bytemuck::cast_slice_mut(&mut v))?;
    Ok(v)
}


pub fn get_first_and_last<T: Display>(data: &[T]) -> String {
    let mut output = String::new();
    if data.len() <= 10 {
        for (i, v) in data.iter().enumerate() {
            output.push_str(&format!("    {}:{}\n", i, v));
        }
    } else {
        for (i, v) in data[..10].iter().enumerate() {
            output.push_str(&format!("    {}:{}\n", i, v));
        }
        for (i, v) in data[data.len()-10..].iter().enumerate() {
            output.push_str(&format!("    {}:{}\n", i + data.len() - 10, v));
        }
    }
    output.push('\n');
    output
}

pub const fn ceil_to_multiple(n: usize, base: usize) -> usize {
    (n + base - 1) / base * base
}