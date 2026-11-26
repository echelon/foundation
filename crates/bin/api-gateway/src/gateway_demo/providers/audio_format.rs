use super::super::artifacts::FetchedAudio;

pub(super) fn parse(bytes: Vec<u8>, expected_seconds: u16) -> Option<FetchedAudio> {
    if bytes.get(..4)? != b"RIFF"
        || bytes.get(8..12)? != b"WAVE"
        || u32::from_le_bytes(bytes.get(4..8)?.try_into().ok()?) as usize != bytes.len() - 8
    {
        return None;
    }
    let mut remaining = &bytes[12..];
    let mut format = None;
    let mut data_length = None;
    while !remaining.is_empty() {
        let kind = remaining.get(..4)?;
        let length = u32::from_le_bytes(remaining.get(4..8)?.try_into().ok()?) as usize;
        let (body, tail) = remaining.get(8..)?.split_at_checked(length)?;
        remaining = tail.get(length % 2..)?;
        match kind {
            b"fmt " if format.is_none() && data_length.is_none() => format = Some(pcm(body)?),
            b"data" if format.is_some() && data_length.is_none() => data_length = Some(length),
            b"fmt " | b"data" => return None,
            _ => {}
        }
    }
    let (channels, sample_rate_hz) = format?;
    let frames = usize::from(expected_seconds).checked_mul(sample_rate_hz as usize)?;
    if data_length? != frames.checked_mul(usize::from(channels) * 2)? {
        return None;
    }
    Some(FetchedAudio {
        channels,
        sample_rate_hz,
        duration_ms: u32::from(expected_seconds) * 1000,
        bytes,
    })
}

fn pcm(body: &[u8]) -> Option<(u16, u32)> {
    if body.len() != 16 && (body.len() != 18 || body[16..] != [0, 0]) {
        return None;
    }
    let encoding = u16::from_le_bytes(body.get(..2)?.try_into().ok()?);
    let channels = u16::from_le_bytes(body.get(2..4)?.try_into().ok()?);
    let rate = u32::from_le_bytes(body.get(4..8)?.try_into().ok()?);
    let byte_rate = u32::from_le_bytes(body.get(8..12)?.try_into().ok()?);
    let alignment = u16::from_le_bytes(body.get(12..14)?.try_into().ok()?);
    let bits = u16::from_le_bytes(body.get(14..16)?.try_into().ok()?);
    (encoding == 1
        && (1..=2).contains(&channels)
        && (8_000..=48_000).contains(&rate)
        && bits == 16
        && alignment == channels * 2
        && byte_rate == rate * u32::from(alignment))
    .then_some((channels, rate))
}
