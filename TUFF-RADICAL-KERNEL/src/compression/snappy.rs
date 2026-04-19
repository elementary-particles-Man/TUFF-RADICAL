use alloc::vec::Vec;
use core::cmp;

const TAG_LITERAL: u8 = 0x00;
const TAG_COPY_1: u8 = 0x01;
const TAG_COPY_2: u8 = 0x02;
const TAG_COPY_4: u8 = 0x03;
const MIN_MATCH: usize = 4;
const MAX_OFFSET: usize = 65_535;
const SEARCH_WINDOW: usize = 2_048;
const MAX_COPY_LEN: usize = 64;

pub fn compress(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() + input.len() / 6 + 32);
    write_varint(input.len() as u32, &mut out);

    let mut literal_start = 0usize;
    let mut cursor = 0usize;

    while cursor + MIN_MATCH <= input.len() {
        let Some((match_pos, match_len)) = find_match(input, cursor) else {
            cursor += 1;
            continue;
        };

        if literal_start < cursor {
            emit_literal(&input[literal_start..cursor], &mut out);
        }

        emit_copy(cursor - match_pos, match_len, &mut out);
        cursor += match_len;
        literal_start = cursor;
    }

    if literal_start < input.len() {
        emit_literal(&input[literal_start..], &mut out);
    }

    out
}

pub fn decompress(input: &[u8]) -> Result<Vec<u8>, &'static str> {
    let mut cursor = 0usize;
    let expected_len = read_varint(input, &mut cursor)? as usize;
    let mut out = Vec::with_capacity(expected_len);

    while cursor < input.len() && out.len() < expected_len {
        let tag = input[cursor];
        cursor += 1;

        match tag & 0x03 {
            TAG_LITERAL => {
                let len = decode_literal_len(tag, input, &mut cursor)?;
                if cursor + len > input.len() {
                    return Err("literal exceeds input");
                }
                out.extend_from_slice(&input[cursor..cursor + len]);
                cursor += len;
            }
            TAG_COPY_1 => {
                if cursor >= input.len() {
                    return Err("copy1 truncated");
                }
                let len = 4 + ((tag >> 2) & 0x07) as usize;
                let offset = ((((tag as usize) & 0xE0) << 3) | input[cursor] as usize) as usize;
                cursor += 1;
                copy_from_output(&mut out, offset, len)?;
            }
            TAG_COPY_2 => {
                if cursor + 2 > input.len() {
                    return Err("copy2 truncated");
                }
                let len = 1 + (tag >> 2) as usize;
                let offset = u16::from_le_bytes([input[cursor], input[cursor + 1]]) as usize;
                cursor += 2;
                copy_from_output(&mut out, offset, len)?;
            }
            TAG_COPY_4 => {
                if cursor + 4 > input.len() {
                    return Err("copy4 truncated");
                }
                let len = 1 + (tag >> 2) as usize;
                let offset = u32::from_le_bytes([
                    input[cursor],
                    input[cursor + 1],
                    input[cursor + 2],
                    input[cursor + 3],
                ]) as usize;
                cursor += 4;
                copy_from_output(&mut out, offset, len)?;
            }
            _ => unreachable!(),
        }
    }

    if out.len() != expected_len {
        return Err("decompressed length mismatch");
    }

    Ok(out)
}

fn find_match(input: &[u8], cursor: usize) -> Option<(usize, usize)> {
    if cursor + MIN_MATCH > input.len() {
        return None;
    }

    let start = cursor.saturating_sub(SEARCH_WINDOW);
    let mut best: Option<(usize, usize)> = None;

    for pos in start..cursor {
        let offset = cursor - pos;
        if offset > MAX_OFFSET || input[pos] != input[cursor] {
            continue;
        }

        let mut len = 0usize;
        while cursor + len < input.len()
            && input[pos + (len % offset)] == input[cursor + len]
            && len < 256
        {
            len += 1;
        }

        if len >= MIN_MATCH {
            if let Some((_, best_len)) = best {
                if len > best_len {
                    best = Some((pos, len));
                }
            } else {
                best = Some((pos, len));
            }
        }
    }

    best
}

fn emit_literal(mut literal: &[u8], out: &mut Vec<u8>) {
    while !literal.is_empty() {
        let chunk_len = cmp::min(literal.len(), 60 + 0xFF_FF_FF);
        write_literal_tag(chunk_len, out);
        out.extend_from_slice(&literal[..chunk_len]);
        literal = &literal[chunk_len..];
    }
}

fn write_literal_tag(len: usize, out: &mut Vec<u8>) {
    let len_minus_one = len - 1;
    if len < 60 {
        out.push(((len_minus_one as u8) << 2) | TAG_LITERAL);
        return;
    }

    let bytes_needed = if len_minus_one <= 0xFF {
        1
    } else if len_minus_one <= 0xFFFF {
        2
    } else if len_minus_one <= 0xFF_FFFF {
        3
    } else {
        4
    };

    out.push((((59 + bytes_needed) as u8) << 2) | TAG_LITERAL);
    for i in 0..bytes_needed {
        out.push(((len_minus_one >> (i * 8)) & 0xFF) as u8);
    }
}

fn emit_copy(offset: usize, mut len: usize, out: &mut Vec<u8>) {
    while len > 0 {
        let chunk = cmp::min(len, MAX_COPY_LEN);
        if offset < 2048 && (4..=11).contains(&chunk) {
            let tag = (((chunk - 4) as u8) << 2)
                | (((offset >> 8) as u8) << 5)
                | TAG_COPY_1;
            out.push(tag);
            out.push((offset & 0xFF) as u8);
        } else {
            let copy_len = cmp::max(chunk, MIN_MATCH);
            let tag = (((copy_len - 1) as u8) << 2) | TAG_COPY_2;
            out.push(tag);
            out.extend_from_slice(&(offset as u16).to_le_bytes());
            len -= copy_len;
            continue;
        }
        len -= chunk;
    }
}

fn copy_from_output(out: &mut Vec<u8>, offset: usize, len: usize) -> Result<(), &'static str> {
    if offset == 0 || offset > out.len() {
        return Err("invalid copy offset");
    }

    let start = out.len() - offset;
    for i in 0..len {
        let byte = out[start + (i % offset)];
        out.push(byte);
    }

    Ok(())
}

fn write_varint(mut value: u32, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn read_varint(input: &[u8], cursor: &mut usize) -> Result<u32, &'static str> {
    let mut shift = 0u32;
    let mut value = 0u32;

    while *cursor < input.len() && shift <= 28 {
        let byte = input[*cursor];
        *cursor += 1;
        value |= ((byte & 0x7F) as u32) << shift;
        if (byte & 0x80) == 0 {
            return Ok(value);
        }
        shift += 7;
    }

    Err("invalid varint")
}

fn decode_literal_len(tag: u8, input: &[u8], cursor: &mut usize) -> Result<usize, &'static str> {
    let len_code = (tag >> 2) as usize;
    if len_code < 60 {
        return Ok(len_code + 1);
    }

    let bytes = len_code - 59;
    if *cursor + bytes > input.len() {
        return Err("literal length truncated");
    }

    let mut len_minus_one = 0usize;
    for i in 0..bytes {
        len_minus_one |= (input[*cursor + i] as usize) << (i * 8);
    }
    *cursor += bytes;
    Ok(len_minus_one + 1)
}
