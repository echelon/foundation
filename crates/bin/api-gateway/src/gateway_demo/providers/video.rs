use super::super::artifacts::FetchedVideo;

pub(in crate::gateway_demo) const MAX_PROMPT_BYTES: usize = 2048;

pub(super) fn parse(
    bytes: Vec<u8>,
    expected_seconds: u16,
    expected_size: (u32, u32),
) -> Option<FetchedVideo> {
    let mut data = bytes.as_slice();
    let (kind, brands) = next_box(&mut data)?;
    if kind != b"ftyp"
        || brands.len() < 8
        || !matches!(brands.get(..4)?, b"isom" | b"mp41" | b"mp42")
    {
        return None;
    }
    let mut movie = None;
    let mut media_data = false;
    while !data.is_empty() {
        match next_box(&mut data)? {
            (b"moov", body) if movie.is_none() => movie = Some(body),
            (b"moov", _) => return None,
            (b"mdat", body) if !body.is_empty() => media_data = true,
            _ => {}
        }
    }
    let movie = movie.filter(|_| media_data)?;
    let header = child(movie, b"mvhd")?;
    let (scale, ticks) = match header.first()? {
        0 => (word(header, 12)?, u64::from(word(header, 16)?)),
        1 => (
            word(header, 20)?,
            u64::from_be_bytes(header.get(24..32)?.try_into().ok()?),
        ),
        _ => return None,
    };
    let duration_ms =
        u32::try_from(ticks.checked_mul(1000)?.checked_div(u64::from(scale))?).ok()?;
    if duration_ms.abs_diff(u32::from(expected_seconds) * 1000) > 150 {
        return None;
    }
    let track = video_track(movie)?;
    let dimensions = child(track, b"tkhd")?;
    if dimensions.len() < 84 {
        return None;
    }
    let width = word(dimensions, dimensions.len() - 8)? >> 16;
    let height = word(dimensions, dimensions.len() - 4)? >> 16;
    if (width, height) != expected_size {
        return None;
    }
    Some(FetchedVideo {
        width,
        height,
        duration_ms,
        bytes,
    })
}

fn video_track(mut movie: &[u8]) -> Option<&[u8]> {
    while !movie.is_empty() {
        let (kind, track) = next_box(&mut movie)?;
        if kind == b"trak"
            && child(track, b"mdia")
                .and_then(|media| child(media, b"hdlr"))
                .and_then(|handler| handler.get(8..12))
                == Some(b"vide")
        {
            return Some(track);
        }
    }
    None
}

fn child<'a>(mut data: &'a [u8], wanted: &[u8; 4]) -> Option<&'a [u8]> {
    while !data.is_empty() {
        let (kind, body) = next_box(&mut data)?;
        if kind == wanted {
            return Some(body);
        }
    }
    None
}

fn next_box<'a>(data: &mut &'a [u8]) -> Option<(&'a [u8], &'a [u8])> {
    let kind = data.get(4..8)?;
    let (length, header) = match word(data, 0)? {
        0 => (data.len(), 8),
        1 => (
            usize::try_from(u64::from_be_bytes(data.get(8..16)?.try_into().ok()?)).ok()?,
            16,
        ),
        length => (usize::try_from(length).ok()?, 8),
    };
    let body = data.get(header..length)?;
    *data = data.get(length..)?;
    Some((kind, body))
}

fn word(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}
