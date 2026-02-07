//! Network module for MUD server
//!
//! Provides TCP server, client handling, and broadcast functionality.

pub mod broadcaster;
pub mod client;
pub mod server;

pub use broadcaster::Broadcaster;
pub use client::{Client, ClientState};
pub use server::{run_echo_server, run_server};

use bytes::BytesMut;
use std::io;

/// Maximum line length for client input (한 줄/명령). 1024로 상향(붙여넣기 등).
pub const MAX_LENGTH: usize = 1024;

/// Custom delimiter codec for line-based protocol
/// Supports \r\n and \r\000 delimiters (similar to Twisted's LineOnlyReceiver)
/// Decodes input as UTF-8 (EUC-KR 미지원)
#[derive(Debug, Clone)]
pub struct DelimiterCodec {
    buffer: BytesMut,
    max_length: usize,
}

impl DelimiterCodec {
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(1024),
            max_length: MAX_LENGTH,
        }
    }

    pub fn with_max_length(max_length: usize) -> Self {
        Self {
            buffer: BytesMut::with_capacity(1024),
            max_length,
        }
    }

    /// Feed data into the codec and return any complete lines found
    pub fn feed_data(&mut self, data: &[u8]) -> Result<Vec<String>, io::Error> {
        // 한 줄이 max_length를 넘으면: 버퍼를 비우고 Err. (끊긴 데이터가 다음 명령에 섞이는 것 방지)
        if self.buffer.len() + data.len() > self.max_length {
            self.buffer.clear();
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "한 줄이 {}바이트를 넘습니다. (붙여넣기 시 일부만 넣어 주세요.)",
                    self.max_length
                ),
            ));
        }

        self.buffer.extend_from_slice(data);
        self.extract_lines()
    }

    /// Extract complete lines from the buffer
    fn extract_lines(&mut self) -> Result<Vec<String>, io::Error> {
        let mut lines = Vec::new();

        // Delimiters: \r\n and \r\000
        let delimiters: &[&[u8]] = &[b"\r\n", b"\r\000"];

        loop {
            let mut found_delim = None;
            let mut delim_len = 0;

            for delim in delimiters {
                if let Some(pos) = self.buffer.windows(delim.len()).position(|w| w == *delim) {
                    found_delim = Some(pos);
                    delim_len = delim.len();
                    break;
                }
            }

            match found_delim {
                Some(pos) => {
                    let line_data = self.buffer.split_to(pos + delim_len);
                    let line_bytes = &line_data[..pos];

                    // Decode as UTF-8 (EUC-KR 미지원)
                    let line = String::from_utf8_lossy(line_bytes);
                    lines.push(line.into_owned());
                }
                None => break,
            }
        }

        // 구분자 없이 버퍼만 max_length를 넘은 경우(이론상): 비우고 Err
        if self.buffer.len() > self.max_length {
            self.buffer.clear();
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("한 줄이 {}바이트를 넘습니다.", self.max_length),
            ));
        }

        Ok(lines)
    }

    /// Get the current buffer length
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl Default for DelimiterCodec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delimiter_parsing() {
        let mut codec = DelimiterCodec::new();

        // Test \r\n delimiter
        let lines = codec.feed_data(b"hello\r\n").unwrap();
        assert_eq!(lines, vec!["hello"]);

        // Test \r\000 delimiter
        let lines = codec.feed_data(b"world\r\000").unwrap();
        assert_eq!(lines, vec!["world"]);
    }

    #[test]
    fn test_buffer_accumulation() {
        let mut codec = DelimiterCodec::new();

        // Feed partial data
        let lines = codec.feed_data(b"hel").unwrap();
        assert!(lines.is_empty());

        // Feed rest of data
        let lines = codec.feed_data(b"lo\r\n").unwrap();
        assert_eq!(lines, vec!["hello"]);
    }
}
