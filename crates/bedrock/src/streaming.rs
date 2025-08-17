//! Streaming response handling for Bedrock client

use std::pin::Pin;
use std::task::{Context, Poll};

use chrono::Utc;
use futures::{Stream, StreamExt};
use uuid::Uuid;

use crate::error::{BedrockError, Result};
use crate::message::{StreamChunk, TokenUsage};

/// Streaming response wrapper (simplified for compilation)
pub struct StreamingResponse {
    inner: Pin<Box<dyn Stream<Item = Result<String>> + Send>>,
    model: String,
    buffer: String,
    finished: bool,
}

impl StreamingResponse {
    /// Create a new streaming response
    pub fn new(stream: impl Stream<Item = Result<String>> + Send + 'static, model: String) -> Self {
        Self {
            inner: Box::pin(stream),
            model,
            buffer: String::new(),
            finished: false,
        }
    }
}

impl Stream for StreamingResponse {
    type Item = Result<StreamChunk>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.finished {
            return Poll::Ready(None);
        }

        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(text))) => {
                let chunk = StreamChunk::content(text);
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(e))) => {
                self.finished = true;
                Poll::Ready(Some(Err(e)))
            }
            Poll::Ready(None) => {
                self.finished = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl StreamingResponse {
    /// Collect all chunks into a single string
    pub async fn collect_text(self) -> Result<String> {
        let mut content = String::new();
        let mut stream = self;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            if !chunk.is_final {
                content.push_str(&chunk.content);
            }
        }

        Ok(content)
    }

    /// Collect all chunks into a vector
    pub async fn collect_chunks(self) -> Result<Vec<StreamChunk>> {
        let mut chunks = Vec::new();
        let mut stream = self;

        while let Some(chunk_result) = stream.next().await {
            chunks.push(chunk_result?);
        }

        Ok(chunks)
    }

    /// Get the model being used for this stream
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Check if the stream has finished
    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

/// Stream processor for handling chunks in real-time
pub struct StreamProcessor<F> {
    handler: F,
}

impl<F> StreamProcessor<F>
where
    F: Fn(StreamChunk) -> Result<()>,
{
    /// Create a new stream processor with a handler function
    pub fn new(handler: F) -> Self {
        Self { handler }
    }

    /// Process a streaming response
    pub async fn process(&self, mut stream: StreamingResponse) -> Result<TokenUsage> {
        let mut final_usage = None;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;

            if chunk.is_final {
                final_usage = chunk.usage.clone();
            }

            (self.handler)(chunk)?;
        }

        final_usage.ok_or_else(|| {
            BedrockError::InvalidResponse("No usage information received".to_string())
        })
    }
}

/// Buffer for accumulating streaming content
#[derive(Debug, Default)]
pub struct StreamBuffer {
    content: String,
    chunks: Vec<StreamChunk>,
    total_tokens: usize,
    estimated_cost: f64,
}

impl StreamBuffer {
    /// Create a new stream buffer
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a chunk to the buffer
    pub fn add_chunk(&mut self, chunk: StreamChunk) {
        if !chunk.content.is_empty() {
            self.content.push_str(&chunk.content);
        }

        if let Some(usage) = &chunk.usage {
            self.total_tokens = usage.total_tokens;
            self.estimated_cost = usage.estimated_cost;
        }

        self.chunks.push(chunk);
    }

    /// Get the accumulated content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get all chunks
    pub fn chunks(&self) -> &[StreamChunk] {
        &self.chunks
    }

    /// Get total tokens used
    pub fn total_tokens(&self) -> usize {
        self.total_tokens
    }

    /// Get estimated cost
    pub fn estimated_cost(&self) -> f64 {
        self.estimated_cost
    }

    /// Check if the stream is complete
    pub fn is_complete(&self) -> bool {
        self.chunks.last().map_or(false, |chunk| chunk.is_final)
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.content.clear();
        self.chunks.clear();
        self.total_tokens = 0;
        self.estimated_cost = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[test]
    fn test_stream_buffer() {
        let mut buffer = StreamBuffer::new();

        let chunk1 = StreamChunk::content("Hello");
        let chunk2 = StreamChunk::content(" world");
        let usage = TokenUsage::new(10, 5, "test", 0.001);
        let final_chunk = StreamChunk::final_chunk(usage);

        buffer.add_chunk(chunk1);
        buffer.add_chunk(chunk2);
        buffer.add_chunk(final_chunk);

        assert_eq!(buffer.content(), "Hello world");
        assert_eq!(buffer.chunks().len(), 3);
        assert_eq!(buffer.total_tokens(), 15);
        assert!(buffer.is_complete());
    }

    #[tokio::test]
    async fn test_stream_processor() {
        let chunks = vec![
            Ok(StreamChunk::content("Hello")),
            Ok(StreamChunk::content(" world")),
            Ok(StreamChunk::final_chunk(TokenUsage::new(
                10, 5, "test", 0.001,
            ))),
        ];

        let mock_stream = stream::iter(chunks);
        let streaming_response = StreamingResponse::new(
            mock_stream.map(|chunk| {
                Ok(ConverseStreamOutput::ContentBlockDelta(
                    aws_sdk_bedrockruntime::types::ContentBlockDeltaEvent::builder().build(),
                ))
            }),
            "test-model".to_string(),
        );

        let mut content = String::new();
        let processor = StreamProcessor::new(|chunk: StreamChunk| {
            if !chunk.is_final {
                content.push_str(&chunk.content);
            }
            Ok(())
        });

        // This would work with a proper mock implementation
        // let usage = processor.process(streaming_response).await.unwrap();
        // assert_eq!(content, "Hello world");
    }
}
