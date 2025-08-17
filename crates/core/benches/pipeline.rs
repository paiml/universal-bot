//! Pipeline performance benchmarks
//!
//! These benchmarks measure the performance of the message processing pipeline
//! under various conditions to ensure optimal performance.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

use parking_lot::RwLock;
use universal_bot_core::{
    context::ContextManager, message::MessageType, pipeline::PipelineContext, BotConfig, Context,
    Message, MessagePipeline,
};

/// Benchmark pipeline creation
fn bench_pipeline_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("pipeline_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let config = black_box(BotConfig::default());
            let _pipeline = MessagePipeline::new(&config).await.unwrap();
        });
    });
}

/// Benchmark message processing with different message sizes
fn bench_message_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Setup pipeline and context once
    let config = BotConfig::default();
    let pipeline = rt.block_on(async { MessagePipeline::new(&config).await.unwrap() });

    let context_manager = rt.block_on(async {
        ContextManager::new(config.context_config.clone())
            .await
            .unwrap()
    });

    let context = rt.block_on(async { context_manager.get_or_create("benchmark").await.unwrap() });

    let message_sizes = vec![10, 100, 1000, 5000];

    let mut group = c.benchmark_group("message_processing");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));

    for size in message_sizes {
        let content = "a".repeat(size);
        let message = Message::text(content);

        group.bench_with_input(BenchmarkId::new("size", size), &message, |b, msg| {
            b.to_async(&rt).iter(|| async {
                let msg = black_box(msg.clone());
                let ctx = black_box(context.clone());
                let _response = pipeline.process(msg, ctx).await.unwrap();
            });
        });
    }

    group.finish();
}

/// Benchmark different message types
fn bench_message_types(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let config = BotConfig::default();
    let pipeline = rt.block_on(async { MessagePipeline::new(&config).await.unwrap() });

    let context_manager = rt.block_on(async {
        ContextManager::new(config.context_config.clone())
            .await
            .unwrap()
    });

    let context = rt.block_on(async { context_manager.get_or_create("benchmark").await.unwrap() });

    let message_types = vec![
        ("text", MessageType::Text),
        ("command", MessageType::Command),
        ("system", MessageType::System),
    ];

    let mut group = c.benchmark_group("message_types");

    for (name, msg_type) in message_types {
        let message = Message::with_type("Test message content", msg_type);

        group.bench_function(name, |b| {
            b.to_async(&rt).iter(|| async {
                let msg = black_box(message.clone());
                let ctx = black_box(context.clone());
                let _response = pipeline.process(msg, ctx).await.unwrap();
            });
        });
    }

    group.finish();
}

/// Benchmark concurrent message processing
fn bench_concurrent_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let config = BotConfig::default();
    let pipeline = Arc::new(rt.block_on(async { MessagePipeline::new(&config).await.unwrap() }));

    let context_manager = Arc::new(rt.block_on(async {
        ContextManager::new(config.context_config.clone())
            .await
            .unwrap()
    }));

    let concurrency_levels = vec![1, 5, 10, 20];

    let mut group = c.benchmark_group("concurrent_processing");
    group.sample_size(30);

    for concurrency in concurrency_levels {
        group.bench_with_input(
            BenchmarkId::new("concurrency", concurrency),
            &concurrency,
            |b, &level| {
                b.to_async(&rt).iter(|| async {
                    let pipeline = black_box(pipeline.clone());
                    let context_manager = black_box(context_manager.clone());

                    let mut handles = Vec::new();

                    for i in 0..level {
                        let p = pipeline.clone();
                        let cm = context_manager.clone();

                        let handle = tokio::spawn(async move {
                            let message = Message::text(format!("Message {}", i));
                            let context = cm.get_or_create(&format!("bench-{}", i)).await.unwrap();
                            p.process(message, context).await.unwrap()
                        });

                        handles.push(handle);
                    }

                    // Wait for all tasks to complete
                    for handle in handles {
                        handle.await.unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark context operations
fn bench_context_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let config = BotConfig::default();
    let context_manager = rt.block_on(async {
        ContextManager::new(config.context_config.clone())
            .await
            .unwrap()
    });

    let mut group = c.benchmark_group("context_operations");

    // Benchmark context creation
    group.bench_function("context_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let id = black_box(format!("bench-{}", uuid::Uuid::new_v4()));
            let _context = context_manager.get_or_create(&id).await.unwrap();
        });
    });

    // Benchmark context with message history
    let context = rt.block_on(async {
        context_manager
            .get_or_create("history-bench")
            .await
            .unwrap()
    });

    // Add some history
    for i in 0..100 {
        let message = Message::text(format!("History message {}", i));
        context.write().add_message(&message);
    }

    group.bench_function("context_with_history", |b| {
        b.to_async(&rt).iter(|| async {
            let ctx = black_box(context.clone());
            let message = black_box(Message::text("New message"));
            ctx.write().add_message(&message);
        });
    });

    group.finish();
}

/// Benchmark message serialization/deserialization
fn bench_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_serialization");

    let message = Message::text("Test message for serialization benchmarks")
        .with_conversation_id("bench-conversation")
        .with_user_id("bench-user")
        .with_metadata("key1", serde_json::json!("value1"))
        .with_metadata("key2", serde_json::json!({"nested": "object"}));

    group.bench_function("serialize_json", |b| {
        b.iter(|| {
            let msg = black_box(&message);
            let _json = serde_json::to_string(msg).unwrap();
        });
    });

    let serialized = serde_json::to_string(&message).unwrap();

    group.bench_function("deserialize_json", |b| {
        b.iter(|| {
            let json = black_box(&serialized);
            let _msg: Message = serde_json::from_str(json).unwrap();
        });
    });

    group.bench_function("serialize_vec", |b| {
        b.iter(|| {
            let msg = black_box(&message);
            let _bytes = serde_json::to_vec(msg).unwrap();
        });
    });

    let serialized_bytes = serde_json::to_vec(&message).unwrap();

    group.bench_function("deserialize_vec", |b| {
        b.iter(|| {
            let bytes = black_box(&serialized_bytes);
            let _msg: Message = serde_json::from_slice(bytes).unwrap();
        });
    });

    group.finish();
}

/// Benchmark memory usage patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_patterns");
    group.sample_size(20);

    // Benchmark large context handling
    group.bench_function("large_context", |b| {
        b.to_async(&rt).iter(|| async {
            let config = BotConfig::default();
            let context_manager = ContextManager::new(config.context_config.clone())
                .await
                .unwrap();
            let context = context_manager
                .get_or_create("large-context")
                .await
                .unwrap();

            // Add many messages to create large context
            for i in 0..1000 {
                let message = Message::text(format!("Large context message {}", i));
                context.write().add_message(&message);
            }

            // Simulate processing with large context
            let message = Message::text("Process with large context");
            context.write().add_message(&message);
        });
    });

    // Benchmark rapid context switching
    group.bench_function("context_switching", |b| {
        b.to_async(&rt).iter(|| async {
            let config = BotConfig::default();
            let context_manager = ContextManager::new(config.context_config.clone())
                .await
                .unwrap();

            // Rapidly switch between different contexts
            for i in 0..50 {
                let context_id = format!("switch-{}", i % 10);
                let context = context_manager.get_or_create(&context_id).await.unwrap();
                let message = Message::text(format!("Switch message {}", i));
                context.write().add_message(&message);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_pipeline_creation,
    bench_message_processing,
    bench_message_types,
    bench_concurrent_processing,
    bench_context_operations,
    bench_message_serialization,
    bench_memory_patterns,
);

criterion_main!(benches);
