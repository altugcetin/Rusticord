use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;

pub const UPLOAD_CHUNK_BYTES: usize = 65_536;

#[derive(Clone, Debug)]
pub struct UploadFile {
    pub field: String,
    pub file_name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UploadProgress {
    pub sent: u64,
    pub total: u64,
}

#[derive(Clone, Debug, Default)]
pub struct CancelFlag {
    inner: Arc<AtomicBool>,
}

impl CancelFlag {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }
}

pub fn multipart_content_type(boundary: &str) -> String {
    let mut value = String::from("multipart/form-data; boundary=");
    value.push_str(boundary);
    value
}

pub fn encode_multipart(boundary: &str, payload_json: &str, files: &[UploadFile]) -> Vec<u8> {
    let mut body = Vec::new();
    append_dash_boundary(&mut body, boundary);
    append_str(
        &mut body,
        "\r\nContent-Disposition: form-data; name=\"payload_json\"\r\n\r\n",
    );
    append_str(&mut body, payload_json);
    append_str(&mut body, "\r\n");
    for file in files {
        append_dash_boundary(&mut body, boundary);
        append_str(&mut body, "\r\nContent-Disposition: form-data; name=\"");
        append_str(&mut body, &file.field);
        append_str(&mut body, "\"; filename=\"");
        append_str(&mut body, &sanitize_filename(&file.file_name));
        append_str(&mut body, "\"\r\nContent-Type: ");
        append_str(&mut body, &file.mime);
        append_str(&mut body, "\r\n\r\n");
        body.extend_from_slice(&file.bytes);
        append_str(&mut body, "\r\n");
    }
    append_dash_boundary(&mut body, boundary);
    append_str(&mut body, "--\r\n");
    body
}

pub fn make_boundary(unique: u128) -> String {
    let mut boundary = String::from("------------Rusticord");
    boundary.push_str(&unique.to_string());
    boundary
}

fn append_dash_boundary(body: &mut Vec<u8>, boundary: &str) {
    append_str(body, "--");
    append_str(body, boundary);
}

fn append_str(body: &mut Vec<u8>, text: &str) {
    body.extend_from_slice(text.as_bytes());
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|ch| match ch {
            '"' | '\\' | '\r' | '\n' => '_',
            other => other,
        })
        .collect()
}

pub struct ProgressBody {
    data: Vec<u8>,
    offset: usize,
    total: u64,
    on_progress: Option<Arc<dyn Fn(UploadProgress) + Send + Sync>>,
    cancel: Option<CancelFlag>,
}

impl ProgressBody {
    pub fn new(
        data: Vec<u8>,
        on_progress: Option<Arc<dyn Fn(UploadProgress) + Send + Sync>>,
        cancel: Option<CancelFlag>,
    ) -> Self {
        let total = u64::try_from(data.len()).unwrap_or(u64::MAX);
        Self {
            data,
            offset: 0,
            total,
            on_progress,
            cancel,
        }
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    pub(crate) fn take_chunk(&mut self) -> Option<Result<Bytes, io::Error>> {
        if self.cancel.as_ref().is_some_and(CancelFlag::is_cancelled) {
            return Some(Err(io::Error::other("cancelled")));
        }
        if self.offset >= self.data.len() {
            return None;
        }
        let end = self
            .offset
            .saturating_add(UPLOAD_CHUNK_BYTES)
            .min(self.data.len());
        let slice = self.data.get(self.offset..end)?;
        let bytes = Bytes::copy_from_slice(slice);
        self.offset = end;
        let sent = u64::try_from(self.offset).unwrap_or(u64::MAX);
        if let Some(on_progress) = &self.on_progress {
            on_progress(UploadProgress {
                sent,
                total: self.total,
            });
        }
        Some(Ok(bytes))
    }
}

impl Stream for ProgressBody {
    type Item = Result<Bytes, io::Error>;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.get_mut().take_chunk())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CancelFlag, ProgressBody, UploadFile, UploadProgress, encode_multipart, make_boundary,
        multipart_content_type,
    };
    use std::sync::{Arc, Mutex};

    #[test]
    fn multipart_body_contains_json_and_file_bytes() {
        let files = [UploadFile {
            field: String::from("files[0]"),
            file_name: String::from("note.txt"),
            mime: String::from("text/plain"),
            bytes: Vec::from(b"hello"),
        }];
        let body = encode_multipart("abc", "{\"content\":\"hi\"}", &files);
        let text = String::from_utf8(body).unwrap();
        assert!(text.contains("name=\"payload_json\""));
        assert!(text.contains("{\"content\":\"hi\"}"));
        assert!(text.contains("filename=\"note.txt\""));
        assert!(text.contains("hello"));
        assert!(text.ends_with("--abc--\r\n"));
    }

    #[test]
    fn content_type_uses_boundary() {
        let value = multipart_content_type("abc");
        assert_eq!(value, "multipart/form-data; boundary=abc");
    }

    #[test]
    fn boundary_includes_unique_suffix() {
        assert_eq!(make_boundary(9), "------------Rusticord9");
    }

    #[test]
    fn progress_body_reports_full_send_and_honours_cancel() {
        let reports = Arc::new(Mutex::new(Vec::new()));
        let reports_clone = reports.clone();
        let mut body = ProgressBody::new(
            vec![1, 2, 3, 4],
            Some(Arc::new(move |progress: UploadProgress| {
                reports_clone.lock().unwrap().push(progress);
            })),
            None,
        );
        let first = body.take_chunk();
        assert!(first.is_some_and(|chunk| chunk.unwrap().len() == 4));
        assert!(body.take_chunk().is_none());
        let recorded = reports.lock().unwrap();
        assert_eq!(recorded.as_slice(), [UploadProgress { sent: 4, total: 4 }]);

        let cancel = CancelFlag::new();
        cancel.cancel();
        let mut cancelled = ProgressBody::new(vec![1, 2, 3], None, Some(cancel));
        let result = cancelled.take_chunk();
        assert!(result.is_some_and(|chunk| chunk.is_err()));
    }
}
