
use core::cmp;

const TAG_LITERAL: u8 = 0x00;
const TAG_COPY_1: u8 = 0x01;
const TAG_COPY_2: u8 = 0x02;
const TAG_COPY_4: u8 = 0x03;
const MIN_MATCH: usize = 4;
const MAX_OFFSET: usize = 65_535;
const SEARCH_WINDOW: usize = 2_048;
const MAX_COPY_LEN: usize = 64;

pub fn max_compressed_len(source_len: usize) -> usize {
    32 + source_len + source_len / 6
}

pub fn compress(input: &[u8], output: &mut [u8]) -> Result<usize, &'static str> {
    let mut out_idx = 0;
    
    // Varint length header
    let mut val = input.len() as u32;
    while val >= 0x80 {
        if out_idx >= output.len() { return Err("output too small"); }
        output[out_idx] = (val as u8) | 0x80;
        val >>= 7;
        out_idx += 1;
    }
    if out_idx >= output.len() { return Err("output too small"); }
    output[out_idx] = val as u8;
    out_idx += 1;

    let mut literal_start = 0usize;
    let mut cursor = 0usize;

    while cursor + MIN_MATCH <= input.len() {
        let Some((match_pos, match_len)) = find_match(input, cursor) else {
            cursor += 1;
            continue;
        };

        if literal_start < cursor {
            out_idx = emit_literal_to_buf(&input[literal_start..cursor], output, out_idx)?;
        }

        out_idx = emit_copy_to_buf(cursor - match_pos, match_len, output, out_idx)?;
        cursor += match_len;
        literal_start = cursor;
    }

    if literal_start < input.len() {
        out_idx = emit_literal_to_buf(&input[literal_start..], output, out_idx)?;
    }

    Ok(out_idx)
}

pub fn decompress(input: &[u8], output: &mut [u8]) -> Result<usize, &'static str> {
    let mut cursor = 0usize;
    let expected_len = read_varint(input, &mut cursor)? as usize;
    if expected_len > output.len() { return Err("output buffer too small for expected data"); }
    
    let mut out_idx = 0usize;

    while cursor < input.len() && out_idx < expected_len {
        let tag = input[cursor];
        cursor += 1;

        match tag & 0x03 {
            TAG_LITERAL => {
                let len = decode_literal_len(tag, input, &mut cursor)?;
                if cursor + len > input.len() || out_idx + len > output.len() {
                    return Err("literal exceeds bounds");
                }
                output[out_idx..out_idx + len].copy_from_slice(&input[cursor..cursor + len]);
                cursor += len;
                out_idx += len;
            }
            TAG_COPY_1 => {
                if cursor >= input.len() { return Err("copy1 truncated"); }
                let len = 4 + ((tag >> 2) & 0x07) as usize;
                let offset = ((((tag as usize) & 0xE0) << 3) | input[cursor] as usize) as usize;
                cursor += 1;
                out_idx = copy_from_output_to_buf(output, out_idx, offset, len)?;
            }
            TAG_COPY_2 => {
                if cursor + 2 > input.len() { return Err("copy2 truncated"); }
                let len = 1 + (tag >> 2) as usize;
                let offset = u16::from_le_bytes([input[cursor], input[cursor + 1]]) as usize;
                cursor += 2;
                out_idx = copy_from_output_to_buf(output, out_idx, offset, len)?;
            }
            TAG_COPY_4 => {
                if cursor + 4 > input.len() { return Err("copy4 truncated"); }
                let len = 1 + (tag >> 2) as usize;
                let offset = u32::from_le_bytes([
                    input[cursor], input[cursor + 1], input[cursor + 2], input[cursor + 3],
                ]) as usize;
                cursor += 4;
                out_idx = copy_from_output_to_buf(output, out_idx, offset, len)?;
            }
            _ => unreachable!(),
        }
    }

    Ok(out_idx)
}

fn find_match(input: &[u8], cursor: usize) -> Option<(usize, usize)> {
    if cursor + MIN_MATCH > input.len() { return None; }
    let start = cursor.saturating_sub(SEARCH_WINDOW);
    let mut best: Option<(usize, usize)> = None;
    for pos in start..cursor {
        let offset = cursor - pos;
        if offset > MAX_OFFSET || input[pos] != input[cursor] { continue; }
        let mut len = 0usize;
        while cursor + len < input.len()
            && input[pos + (len % offset)] == input[cursor + len]
            && len < 256
        { len += 1; }
        if len >= MIN_MATCH {
            if let Some((_, best_len)) = best {
                if len > best_len { best = Some((pos, len)); }
            } else { best = Some((pos, len)); }
        }
    }
    best
}

fn emit_literal_to_buf(literal: &[u8], output: &mut [u8], mut out_idx: usize) -> Result<usize, &'static str> {
    let mut src_idx = 0;
    while src_idx < literal.len() {
        let chunk_len = cmp::min(literal.len() - src_idx, 60 + 0xFF_FF_FF);
        out_idx = write_literal_tag_to_buf(chunk_len, output, out_idx)?;
        if out_idx + chunk_len > output.len() { return Err("output small for literal"); }
        output[out_idx..out_idx + chunk_len].copy_from_slice(&literal[src_idx..src_idx + chunk_len]);
        out_idx += chunk_len;
        src_idx += chunk_len;
    }
    Ok(out_idx)
}

fn write_literal_tag_to_buf(len: usize, output: &mut [u8], mut out_idx: usize) -> Result<usize, &'static str> {
    let len_minus_one = len - 1;
    if len < 60 {
        if out_idx >= output.len() { return Err("tag space fail"); }
        output[out_idx] = ((len_minus_one as u8) << 2) | TAG_LITERAL;
        return Ok(out_idx + 1);
    }
    let bytes_needed = if len_minus_one <= 0xFF { 1 } 
                        else if len_minus_one <= 0xFFFF { 2 } 
                        else if len_minus_one <= 0xFF_FFFF { 3 } 
                        else { 4 };
    if out_idx + 1 + bytes_needed > output.len() { return Err("tag size fail"); }
    output[out_idx] = (((59 + bytes_needed) as u8) << 2) | TAG_LITERAL;
    out_idx += 1;
    for i in 0..bytes_needed {
        output[out_idx] = ((len_minus_one >> (i * 8)) & 0xFF) as u8;
        out_idx += 1;
    }
    Ok(out_idx)
}

fn emit_copy_to_buf(offset: usize, mut len: usize, output: &mut [u8], mut out_idx: usize) -> Result<usize, &'static str> {
    while len > 0 {
        let chunk = cmp::min(len, MAX_COPY_LEN);
        if offset < 2048 && (4..=11).contains(&chunk) {
            if out_idx + 2 > output.len() { return Err("copy1 size fail"); }
            output[out_idx] = (((chunk - 4) as u8) << 2) | (((offset >> 8) as u8) << 5) | TAG_COPY_1;
            output[out_idx + 1] = (offset & 0xFF) as u8;
            out_idx += 2;
        } else {
            let copy_len = cmp::max(chunk, MIN_MATCH);
            if out_idx + 3 > output.len() { return Err("copy2 size fail"); }
            output[out_idx] = (((copy_len - 1) as u8) << 2) | TAG_COPY_2;
            let off_bytes = (offset as u16).to_le_bytes();
            output[out_idx + 1] = off_bytes[0];
            output[out_idx + 2] = off_bytes[1];
            out_idx += 3;
            len = len.saturating_sub(copy_len);
            continue;
        }
        len -= chunk;
    }
    Ok(out_idx)
}

fn copy_from_output_to_buf(output: &mut [u8], out_idx: usize, offset: usize, len: usize) -> Result<usize, &'static str> {
    if offset == 0 || offset > out_idx { return Err("invalid copy offset"); }
    if out_idx + len > output.len() { return Err("copy exceeds buffer"); }
    let start = out_idx - offset;
    for i in 0..len {
        output[out_idx + i] = output[start + (i % offset)];
    }
    Ok(out_idx + len)
}

fn read_varint(input: &[u8], cursor: &mut usize) -> Result<u32, &'static str> {
    let mut shift = 0u32;
    let mut value = 0u32;
    while *cursor < input.len() && shift <= 28 {
        let byte = input[*cursor];
        *cursor += 1;
        value |= ((byte & 0x7F) as u32) << shift;
        if (byte & 0x80) == 0 { return Ok(value); }
        shift += 7;
    }
    Err("invalid varint")
}

fn decode_literal_len(tag: u8, input: &[u8], cursor: &mut usize) -> Result<usize, &'static str> {
    let len_code = (tag >> 2) as usize;
    if len_code < 60 { return Ok(len_code + 1); }
    let bytes = len_code - 59;
    if *cursor + bytes > input.len() { return Err("literal length truncated"); }
    let mut len_minus_one = 0usize;
    for i in 0..bytes {
        len_minus_one |= (input[*cursor + i] as usize) << (i * 8);
    }
    *cursor += bytes;
    Ok(len_minus_one + 1)
}
