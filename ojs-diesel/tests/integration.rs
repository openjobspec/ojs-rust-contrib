use ojs_diesel::OutboxPublisher;

#[tokio::test]
async fn test_publisher_creation() {
    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    let publisher = OutboxPublisher::new(client, "postgres://localhost/test")
        .poll_interval(std::time::Duration::from_secs(5))
        .batch_size(50)
        .max_retries(3);

    drop(publisher);
}

#[test]
fn test_outbox_entry_fields() {
    let entry = ojs_diesel::OutboxEntry {
        id: uuid::Uuid::now_v7(),
        job_type: "email.send".to_string(),
        args: serde_json::json!({"to": "user@example.com"}),
        queue: Some("email".to_string()),
        priority: 5,
        status: "pending".to_string(),
        created_at: chrono::Utc::now(),
        published_at: None,
        error_message: None,
        retry_count: 0,
        last_error_at: None,
    };

    assert_eq!(entry.job_type, "email.send");
    assert_eq!(entry.status, "pending");
    assert_eq!(entry.queue, Some("email".to_string()));
    assert_eq!(entry.priority, 5);
    assert_eq!(entry.retry_count, 0);
    assert!(entry.published_at.is_none());
    assert!(entry.error_message.is_none());
}

#[test]
fn test_enqueue_options_default() {
    let opts = ojs_diesel::EnqueueOptions::default();
    assert!(opts.queue.is_none());
    assert_eq!(opts.priority, 0);
}

#[test]
fn test_new_outbox_entry() {
    let entry = ojs_diesel::NewOutboxEntry {
        id: uuid::Uuid::now_v7(),
        job_type: "report.generate".to_string(),
        args: serde_json::json!({"id": 42}),
        queue: Some("reports".to_string()),
        priority: 10,
        status: "pending".to_string(),
        created_at: chrono::Utc::now(),
    };

    assert_eq!(entry.job_type, "report.generate");
    assert_eq!(entry.priority, 10);
}

#[test]
fn test_outbox_entry_serialization_round_trip() {
    let args = serde_json::json!({
        "to": "user@example.com",
        "subject": "Hello",
        "nested": {"key": [1, 2, 3]}
    });

    let entry = ojs_diesel::OutboxEntry {
        id: uuid::Uuid::now_v7(),
        job_type: "email.send".to_string(),
        args: args.clone(),
        queue: None,
        priority: 0,
        status: "pending".to_string(),
        created_at: chrono::Utc::now(),
        published_at: None,
        error_message: None,
        retry_count: 0,
        last_error_at: None,
    };

    // Verify JSON args survive round-trip
    let serialized = serde_json::to_string(&entry.args).unwrap();
    let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, args);
    assert_eq!(deserialized["nested"]["key"][1], 2);
}

#[test]
fn test_outbox_entry_with_no_queue_defaults() {
    let entry = ojs_diesel::OutboxEntry {
        id: uuid::Uuid::now_v7(),
        job_type: "noop".to_string(),
        args: serde_json::json!({}),
        queue: None,
        priority: 0,
        status: "pending".to_string(),
        created_at: chrono::Utc::now(),
        published_at: None,
        error_message: None,
        retry_count: 0,
        last_error_at: None,
    };

    assert!(entry.queue.is_none());
    assert_eq!(entry.priority, 0);
    assert!(entry.published_at.is_none());
}

#[test]
fn test_outbox_entry_error_fields() {
    let now = chrono::Utc::now();
    let entry = ojs_diesel::OutboxEntry {
        id: uuid::Uuid::now_v7(),
        job_type: "fail.job".to_string(),
        args: serde_json::json!({}),
        queue: None,
        priority: 0,
        status: "failed".to_string(),
        created_at: now,
        published_at: None,
        error_message: Some("connection refused".to_string()),
        retry_count: 5,
        last_error_at: Some(now),
    };

    assert_eq!(entry.status, "failed");
    assert_eq!(entry.error_message.as_deref(), Some("connection refused"));
    assert_eq!(entry.retry_count, 5);
    assert!(entry.last_error_at.is_some());
}

#[test]
fn test_batch_new_outbox_entries() {
    let entries: Vec<ojs_diesel::NewOutboxEntry> = (0..10)
        .map(|i| ojs_diesel::NewOutboxEntry {
            id: uuid::Uuid::now_v7(),
            job_type: format!("batch.job.{i}"),
            args: serde_json::json!({"index": i}),
            queue: Some("batch".to_string()),
            priority: i,
            status: "pending".to_string(),
            created_at: chrono::Utc::now(),
        })
        .collect();

    assert_eq!(entries.len(), 10);
    for (i, entry) in entries.iter().enumerate() {
        assert_eq!(entry.job_type, format!("batch.job.{i}"));
        assert_eq!(entry.priority, i as i32);
        assert_eq!(entry.args["index"], i);
    }
}

#[tokio::test]
async fn test_publisher_builder_configuration() {
    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    // Verify builder methods can be chained
    let _publisher = OutboxPublisher::new(client, "postgres://localhost/test")
        .poll_interval(std::time::Duration::from_millis(100))
        .batch_size(200)
        .max_retries(10);
}

#[test]
fn test_enqueue_options_with_custom_values() {
    let opts = ojs_diesel::EnqueueOptions {
        queue: Some("high-priority".to_string()),
        priority: 100,
    };

    assert_eq!(opts.queue, Some("high-priority".to_string()));
    assert_eq!(opts.priority, 100);
}
