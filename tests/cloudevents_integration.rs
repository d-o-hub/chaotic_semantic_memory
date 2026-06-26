#[cfg(feature = "cloudevents")]
use async_trait::async_trait;
#[cfg(feature = "cloudevents")]
use chaotic_semantic_memory::{
    ChaoticSemanticFramework, HVec10240, framework_events_ce::EventEmitter,
};
#[cfg(feature = "cloudevents")]
use cloudevents::{AttributesReader, Event};
#[cfg(feature = "cloudevents")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "cloudevents")]
#[derive(Debug, Default, Clone)]
struct TestEmitter {
    events: Arc<Mutex<Vec<Event>>>,
}

#[cfg(feature = "cloudevents")]
#[async_trait]
impl EventEmitter for TestEmitter {
    fn name(&self) -> &str {
        "test"
    }

    async fn emit(&self, event: Event) -> Result<(), chaotic_semantic_memory::error::MemoryError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

#[cfg(feature = "cloudevents")]
#[tokio::test]
async fn test_cloudevents_emission_lifecycle() {
    let test_emitter = TestEmitter::default();
    let events_collected = test_emitter.events.clone();

    // 1. Build framework with the custom emitter
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_emitter(Arc::new(test_emitter))
        .build()
        .await
        .unwrap();

    // 2. Perform Concept Injection
    framework
        .inject_concept("concept-1", HVec10240::random())
        .await
        .unwrap();

    // 3. Perform Concept Update (requires updating metadata or vector)
    // Let's perform an update of concept metadata
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("test-key".to_string(), serde_json::json!("test-value"));
    framework
        .update_concept_metadata("concept-1", metadata)
        .await
        .unwrap();

    // 4. Inject another concept and associate them
    framework
        .inject_concept("concept-2", HVec10240::random())
        .await
        .unwrap();

    framework
        .associate("concept-1", "concept-2", 0.95)
        .await
        .unwrap();

    // 5. Disassociate them
    framework
        .disassociate("concept-1", "concept-2")
        .await
        .unwrap();

    // 6. Delete concept
    framework.delete_concept("concept-2").await.unwrap();

    // Wait a brief moment to ensure async events are captured if queued (though they are awaited inline)
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Check collected events
    {
        let events = events_collected.lock().unwrap();

        // We expect events for:
        // - ConceptInjected (concept-1)
        // - ConceptUpdated (concept-1 metadata)
        // - ConceptInjected (concept-2)
        // - Associated (concept-1 -> concept-2)
        // - Disassociated (concept-1 -> concept-2)
        // - ConceptDeleted (concept-2)
        // - BindingCreated (concept-1, concept-2)
        assert!(
            events.len() >= 6,
            "Expected at least 6 events, got {}",
            events.len()
        );

        // Verify injected event details
        let inject_event = events
            .iter()
            .find(|e| {
                e.ty() == "io.d-o-hub.csm.memory.injected" && e.subject() == Some("concept-1")
            })
            .expect("ConceptInjected event for concept-1");
        assert!(inject_event.source().contains("chaotic-semantic-memory://"));

        // Verify updated event details
        let update_event = events
            .iter()
            .find(|e| e.ty() == "io.d-o-hub.csm.memory.updated")
            .expect("ConceptUpdated event");
        assert_eq!(update_event.subject(), Some("concept-1"));

        // Verify associated event details
        let assoc_event = events
            .iter()
            .find(|e| e.ty() == "io.d-o-hub.csm.memory.associated")
            .expect("Associated event");
        assert_eq!(assoc_event.subject(), Some("concept-1"));

        // Verify disassociated event details
        let disassoc_event = events
            .iter()
            .find(|e| e.ty() == "io.d-o-hub.csm.memory.disassociated")
            .expect("Disassociated event");
        assert_eq!(disassoc_event.subject(), Some("concept-1"));

        // Verify deleted event details
        let delete_event = events
            .iter()
            .find(|e| e.ty() == "io.d-o-hub.csm.memory.deleted")
            .expect("ConceptDeleted event");
        assert_eq!(delete_event.subject(), Some("concept-2"));
        drop(events);
    }
}

#[cfg(all(feature = "events-http", not(target_arch = "wasm32")))]
#[tokio::test]
async fn test_http_emitter_delivery() {
    use std::io::Read;
    use std::net::TcpListener;

    // Set up a local TCP listener to act as our HTTP receiver
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let local_addr = listener.local_addr().unwrap();
    let url = format!("http://{local_addr}");

    // Spawn a thread to read the incoming POST request
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0; 2048];
        let bytes_read = stream.read(&mut buffer).unwrap();
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);

        // Send back a HTTP 200 OK response
        use std::io::Write;
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).unwrap();

        request.into_owned()
    });

    // Create framework with HttpEmitter
    let http_emitter = chaotic_semantic_memory::framework_events_ce::HttpEmitter::new(url);
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_emitter(Arc::new(http_emitter))
        .build()
        .await
        .unwrap();

    // Trigger an event to be emitted
    framework
        .inject_concept("http-concept", HVec10240::random())
        .await
        .unwrap();

    // Wait for mock HTTP server to process request
    let request_str = tokio::task::spawn_blocking(move || handle.join().unwrap())
        .await
        .unwrap();

    // Assert the HTTP request has CloudEvent payload or type
    assert!(request_str.contains("POST"), "Expected a POST request");
    assert!(
        request_str.contains("io.d-o-hub.csm.memory.injected") || request_str.contains("injected"),
        "Request did not contain expected event type"
    );
}
