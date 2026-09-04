use flate2::{Decompress, FlushDecompress, Status};

use crate::GatewayError;

const SUFFIX: [u8; 4] = [0x00, 0x00, 0xff, 0xff];

pub struct ZlibStreamDecoder {
    inflator: Decompress,
    pending: Vec<u8>,
}

impl Default for ZlibStreamDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ZlibStreamDecoder {
    pub fn new() -> Self {
        Self {
            inflator: Decompress::new(true),
            pending: Vec::new(),
        }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<Vec<u8>>, GatewayError> {
        self.pending.extend_from_slice(chunk);
        let mut messages = Vec::new();
        while let Some(end) = suffix_end(&self.pending) {
            let split_at = end.saturating_add(1);
            if split_at > self.pending.len() {
                return Err(GatewayError::Compression);
            }
            let rest = self.pending.split_off(split_at);
            let frame = std::mem::replace(&mut self.pending, rest);
            messages.push(inflate(&mut self.inflator, &frame)?);
        }
        Ok(messages)
    }
}

fn suffix_end(buffer: &[u8]) -> Option<usize> {
    if buffer.len() < 4 {
        return None;
    }
    let last = buffer.len() - 4;
    (0..=last).find_map(|start| {
        if buffer.get(start..start.saturating_add(4)) == Some(SUFFIX.as_slice()) {
            Some(start.saturating_add(3))
        } else {
            None
        }
    })
}

fn inflate(inflator: &mut Decompress, frame: &[u8]) -> Result<Vec<u8>, GatewayError> {
    let mut output = Vec::new();
    let mut consumed = 0_usize;
    loop {
        let input = frame.get(consumed..).unwrap_or(&[]);
        let start = output.len();
        output.resize(start.saturating_add(4096), 0);
        let dest = output.get_mut(start..).ok_or(GatewayError::Compression)?;
        let before_in = inflator.total_in();
        let before_out = inflator.total_out();
        let status = inflator
            .decompress(input, dest, FlushDecompress::Sync)
            .map_err(|_| GatewayError::Compression)?;
        let read = usize::try_from(inflator.total_in().saturating_sub(before_in)).unwrap_or(0);
        let written = usize::try_from(inflator.total_out().saturating_sub(before_out)).unwrap_or(0);
        output.truncate(start.saturating_add(written));
        consumed = consumed.saturating_add(read);
        match status {
            Status::StreamEnd => break,
            Status::Ok | Status::BufError => {
                if consumed >= frame.len() && written == 0 {
                    break;
                }
                if read == 0 && written == 0 {
                    break;
                }
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::ZlibStreamDecoder;
    use flate2::{Compress, Compression, FlushCompress};

    fn sync_frame(bytes: &[u8]) -> Vec<u8> {
        let mut compress = Compress::new(Compression::default(), true);
        let mut out = Vec::with_capacity(bytes.len().saturating_add(64));
        compress
            .compress_vec(bytes, &mut out, FlushCompress::Sync)
            .unwrap();
        out
    }

    #[test]
    fn inflates_a_complete_sync_flushed_json_frame() {
        let payload = br#"{"op":10,"d":{"heartbeat_interval":41250}}"#;
        let frame = sync_frame(payload);
        let mut decoder = ZlibStreamDecoder::new();
        let messages = decoder.push(&frame).unwrap();
        assert_eq!(messages.as_slice(), [payload.as_slice()]);
    }

    #[test]
    fn inflates_when_bytes_arrive_one_at_a_time() {
        let payload = br#"{"op":11}"#;
        let frame = sync_frame(payload);
        let mut decoder = ZlibStreamDecoder::new();
        let mut found = Vec::new();
        for piece in frame.chunks(1) {
            found.extend(decoder.push(piece).unwrap());
        }
        assert_eq!(found.as_slice(), [payload.as_slice()]);
    }

    #[test]
    fn two_messages_on_one_connection_keep_inflator_state() {
        let mut compress = Compress::new(Compression::default(), true);
        let mut first = Vec::with_capacity(64);
        compress
            .compress_vec(br#"{"op":10}"#, &mut first, FlushCompress::Sync)
            .unwrap();
        let mut second = Vec::with_capacity(64);
        compress
            .compress_vec(br#"{"op":11}"#, &mut second, FlushCompress::Sync)
            .unwrap();
        let mut decoder = ZlibStreamDecoder::new();
        let mut all = decoder.push(&first).unwrap();
        all.extend(decoder.push(&second).unwrap());
        assert_eq!(all.len(), 2);
        assert_eq!(
            all.first().map(Vec::as_slice),
            Some(br#"{"op":10}"#.as_slice())
        );
        assert_eq!(
            all.get(1).map(Vec::as_slice),
            Some(br#"{"op":11}"#.as_slice())
        );
    }
}
