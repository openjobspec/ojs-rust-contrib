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
