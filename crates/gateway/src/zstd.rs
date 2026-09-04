use std::io::{Cursor, Read};

use ruzstd::decoding::StreamingDecoder;

use crate::GatewayError;

const MAX_PENDING: usize = 16 * 1024 * 1024;

pub struct ZstdStreamDecoder {
    pub(crate) pending: Vec<u8>,
}

impl Default for ZstdStreamDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ZstdStreamDecoder {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<Vec<u8>>, GatewayError> {
        if self.pending.len().saturating_add(chunk.len()) > MAX_PENDING {
            return Err(GatewayError::Compression);
        }
        self.pending.extend_from_slice(chunk);
        let cursor = Cursor::new(self.pending.clone());
        let mut decoder = match StreamingDecoder::new(cursor) {
            Ok(decoder) => decoder,
            Err(_) => return Ok(Vec::new()),
        };
        let mut output = Vec::new();
        match decoder.read_to_end(&mut output) {
            Ok(_) if !output.is_empty() => {
                self.pending.clear();
                Ok(vec![output])
            }
            _ => Ok(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ZstdStreamDecoder;

    #[test]
    fn incomplete_input_waits() {
        let mut decoder = ZstdStreamDecoder::new();
        let messages = decoder.push(&[0x28, 0xb5]).unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn rejects_an_overlong_pending_buffer() {
        let mut decoder = ZstdStreamDecoder::new();
        decoder.pending = vec![0; super::MAX_PENDING];
        let error = decoder.push(&[1]).unwrap_err();
        assert!(matches!(error, crate::GatewayError::Compression));
    }
}
