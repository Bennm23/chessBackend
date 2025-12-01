use std::{fmt::Display, io::{self, Read}, ops::{Deref, DerefMut}};

use pleco::{Board, PieceType, Player};

#[repr(align(64))]
pub struct CacheAligned<T>(pub T);

impl<T> Deref for CacheAligned<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for CacheAligned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}


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

pub const fn ceil_to_multiple(x: usize, m: usize) -> usize {
    if x % m == 0 {
        x
    } else {
        x + (m - (x % m))
    }
}

pub fn win_rate_params(board : &Board) -> (f32, f32) {

    let material = 
        board.count_piece(Player::White, PieceType::P) as i32 + board.count_piece(Player::Black, PieceType::P) as i32 +
        3 * (board.count_piece(Player::White, PieceType::N) as i32 + board.count_piece(Player::Black, PieceType::N) as i32) +
        3 * (board.count_piece(Player::White, PieceType::B) as i32 + board.count_piece(Player::Black, PieceType::B) as i32) +
        5 * (board.count_piece(Player::White, PieceType::R) as i32 + board.count_piece(Player::Black, PieceType::R) as i32) +
        9 * (board.count_piece(Player::White, PieceType::Q) as i32 + board.count_piece(Player::Black, PieceType::Q) as i32);

    // The fitted model only uses data for material counts in [17, 78], and is anchored at count 58.
    let m = material.clamp(17, 78) as f32 / 58.0;
    
    // Return a = p_a(material) and b = p_b(material), see github.com/official-stockfish/WDL_model
    let ass = [-13.50030198, 40.92780883, -36.82753545, 386.83004070];
    let bs = [96.53354896, -165.79058388, 90.89679019, 49.29561889];

    let a = (((ass[0] * m + ass[1]) * m + ass[2]) * m) + ass[3];
    let b = (((bs[0] * m + bs[1]) * m + bs[2]) * m) + bs[3];

    return (a, b);
}


fn to_cp(v: i32, board : &Board) -> f32 {
    let (a, b) = win_rate_params(board);
    (100f32 * v as f32 / a).round() 
}

pub fn format_cp_aligned_dot(v: i32, board: &Board) -> String {

    let pawns = (0.01 * to_cp(v, board)).abs();
    
    format!("{} {:.2}", if v < 0 {"-"} else {"+"}, pawns)
}