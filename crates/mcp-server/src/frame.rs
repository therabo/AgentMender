//! A bounded preflight for duplicate JSON object keys before the SDK parses values.
//! The SDK retains ownership of JSON-RPC framing and protocol negotiation.
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::{
    collections::BTreeSet,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, ReadBuf};

const DUPLICATE: &str = "duplicate JSON object key";
struct Check;
impl<'de> DeserializeSeed<'de> for Check {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_any(Check)
    }
}
impl<'de> Visitor<'de> for Check {
    type Value = ();
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("JSON value")
    }
    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: de::Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E: de::Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_string<E: de::Error>(self, _: String) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_none<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<(), A::Error> {
        while a.next_element_seed(Check)?.is_some() {}
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<(), A::Error> {
        let mut keys = BTreeSet::new();
        while let Some(key) = a.next_key::<String>()? {
            if !keys.insert(key) {
                return Err(de::Error::custom(DUPLICATE));
            }
            a.next_value_seed(Check)?;
        }
        Ok(())
    }
}
fn has_duplicate(frame: &[u8]) -> bool {
    let mut d = serde_json::Deserializer::from_slice(frame);
    Check
        .deserialize(&mut d)
        .is_err_and(|e| e.to_string().starts_with(DUPLICATE))
}

pub struct CheckedReader<R> {
    inner: R,
    buffer: Vec<u8>,
    ready: usize,
    eof: bool,
    limit: usize,
}
impl<R> CheckedReader<R> {
    pub fn new(inner: R, limit: usize) -> Self {
        Self {
            inner,
            buffer: Vec::new(),
            ready: 0,
            eof: false,
            limit,
        }
    }
}
impl<R: AsyncRead + Unpin> AsyncRead for CheckedReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        loop {
            if self.ready > 0 {
                let n = self.ready.min(output.remaining());
                output.put_slice(&self.buffer[..n]);
                self.buffer.drain(..n);
                self.ready -= n;
                return Poll::Ready(Ok(()));
            }
            if let Some(end) = self.buffer.iter().position(|b| *b == b'\n') {
                if end > self.limit || has_duplicate(&self.buffer[..end]) {
                    return Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "invalid frame",
                    )));
                }
                self.ready = end + 1;
                continue;
            }
            if self.buffer.len() > self.limit {
                return Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid frame",
                )));
            }
            if self.eof {
                if has_duplicate(&self.buffer) {
                    return Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "invalid frame",
                    )));
                }
                self.ready = self.buffer.len();
                if self.ready == 0 {
                    return Poll::Ready(Ok(()));
                }
                continue;
            }
            let mut bytes = [0u8; 4096];
            let mut read = ReadBuf::new(&mut bytes);
            match Pin::new(&mut self.inner).poll_read(cx, &mut read) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Ready(Ok(())) if read.filled().is_empty() => self.eof = true,
                Poll::Ready(Ok(())) => self.buffer.extend_from_slice(read.filled()),
            }
        }
    }
}
