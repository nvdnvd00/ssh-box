use bytes::{Buf, BytesMut};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
use tokio::sync::mpsc;

pub const FRAME_WINDOW: Duration = Duration::from_millis(8);
pub const MAX_BATCH_BYTES: usize = 64 * 1024;

pub async fn batch_stream_to_sink<R, F>(
    mut reader: R,
    mut on_batch: F,
    mut shutdown_rx: mpsc::Receiver<()>,
) -> std::io::Result<()>
where
    R: AsyncRead + Unpin,
    F: FnMut(Vec<u8>) -> bool,
{
    let mut read_buf = [0u8; 16 * 1024];
    let mut batch = BytesMut::with_capacity(MAX_BATCH_BYTES);
    let sleep = tokio::time::sleep(FRAME_WINDOW);
    tokio::pin!(sleep);
    let mut timer_armed = false;

    loop {
        let max_read = (MAX_BATCH_BYTES - batch.len()).min(read_buf.len());
        tokio::select! {
            biased;
            _ = shutdown_rx.recv() => {
                if !batch.is_empty() {
                    let _ = on_batch(batch.split().to_vec());
                }
                return Ok(());
            }
            _ = &mut sleep, if timer_armed => {
                if !batch.is_empty() && !on_batch(batch.split().to_vec()) {
                    return Ok(());
                }
                timer_armed = false;
            }
            res = reader.read(&mut read_buf[..max_read]) => {
                match res {
                    Ok(0) => {
                        if !batch.is_empty() {
                            let _ = on_batch(batch.split().to_vec());
                        }
                        return Ok(());
                    }
                    Ok(n) => {
                        batch.extend_from_slice(&read_buf[..n]);
                        if batch.len() >= MAX_BATCH_BYTES {
                            if !on_batch(batch.split().to_vec()) {
                                return Ok(());
                            }
                            timer_armed = false;
                        } else if !timer_armed {
                            sleep.as_mut().reset(tokio::time::Instant::now() + FRAME_WINDOW);
                            timer_armed = true;
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        }
    }
}

pub struct ChannelReader {
    rx: mpsc::Receiver<Vec<u8>>,
    rem: BytesMut,
}

impl ChannelReader {
    pub fn new(rx: mpsc::Receiver<Vec<u8>>) -> Self {
        Self {
            rx,
            rem: BytesMut::new(),
        }
    }
}

impl AsyncRead for ChannelReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if !self.rem.is_empty() {
            let to_copy = self.rem.len().min(buf.remaining());
            buf.put_slice(&self.rem[..to_copy]);
            self.rem.advance(to_copy);
            return Poll::Ready(Ok(()));
        }

        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(chunk)) => {
                let to_copy = chunk.len().min(buf.remaining());
                buf.put_slice(&chunk[..to_copy]);
                if to_copy < chunk.len() {
                    self.rem.extend_from_slice(&chunk[to_copy..]);
                }
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn coalesces_large_flood_without_dropping_bytes() {
        let payload: Vec<u8> = (0..(1024 * 1024)).map(|i| (i % 251) as u8).collect();
        let reader = Cursor::new(payload.clone());
        let (_tx, rx) = mpsc::channel(1);
        let mut received = Vec::new();
        let mut batch_count = 0usize;

        batch_stream_to_sink(
            reader,
            |chunk| {
                assert!(!chunk.is_empty());
                assert!(
                    chunk.len() <= MAX_BATCH_BYTES,
                    "chunk size {} exceeded MAX_BATCH_BYTES {}",
                    chunk.len(),
                    MAX_BATCH_BYTES
                );
                batch_count += 1;
                received.extend_from_slice(&chunk);
                true
            },
            rx,
        )
        .await
        .unwrap();

        assert_eq!(received.len(), payload.len());
        assert_eq!(received, payload);
        assert_eq!(batch_count, 16);
    }

    #[tokio::test]
    async fn flushes_partial_frame_after_8ms_window() {
        let (mut writer, reader) = tokio::io::duplex(4096);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        let (batch_tx, mut batch_rx) = mpsc::unbounded_channel::<Vec<u8>>();

        let handle = tokio::spawn(async move {
            batch_stream_to_sink(
                reader,
                move |chunk| batch_tx.send(chunk).is_ok(),
                shutdown_rx,
            )
            .await
        });

        use tokio::io::AsyncWriteExt;
        writer.write_all(b"hello ").await.unwrap();
        writer.write_all(b"world").await.unwrap();

        let chunk = tokio::time::timeout(Duration::from_millis(50), batch_rx.recv())
            .await
            .expect("should flush within 8ms window")
            .expect("should receive coalesced chunk");
        assert_eq!(chunk, b"hello world");

        let _ = shutdown_tx.send(()).await;
        handle.await.unwrap().unwrap();
    }
}
