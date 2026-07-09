#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::significant_drop_tightening)]

#[cfg(any(feature = "embed-openai", feature = "embed-voyage"))]
mod tests {
    use chaotic_semantic_memory::embedding::EmbeddingProvider;
    #[cfg(feature = "embed-openai")]
    use chaotic_semantic_memory::embedding::OpenAiProvider;
    #[cfg(feature = "embed-voyage")]
    use chaotic_semantic_memory::embedding::VoyageProvider;
    use mockito::Server;
    use serde_json::json;

    #[cfg(feature = "embed-openai")]
    #[tokio::test]
    async fn test_openai_embed_mock() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let mock = server
            .mock("POST", "/embeddings")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "data": [
                        {
                            "embedding": [0.1, 0.2, 0.3]
                        }
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let provider = OpenAiProvider::new("test-key".into())
            .unwrap()
            .with_base_url(url);

        let result = provider.embed("hello").await.unwrap();
        assert_eq!(result, vec![0.1, 0.2, 0.3]);
        mock.assert_async().await;
    }

    #[cfg(feature = "embed-openai")]
    #[tokio::test]
    async fn test_openai_embed_batch_mock() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let mock = server
            .mock("POST", "/embeddings")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "data": [
                        { "embedding": [0.1, 0.2] },
                        { "embedding": [0.3, 0.4] }
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let provider = OpenAiProvider::new("test-key".into())
            .unwrap()
            .with_base_url(url);

        let result = provider.embed_batch(&["a", "b"]).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec![0.1, 0.2]);
        assert_eq!(result[1], vec![0.3, 0.4]);
        mock.assert_async().await;
    }

    #[cfg(feature = "embed-voyage")]
    #[tokio::test]
    async fn test_voyage_embed_mock() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let mock = server
            .mock("POST", "/embeddings")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "data": [
                        {
                            "embedding": [0.5, 0.6]
                        }
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let provider = VoyageProvider::new("test-key".into())
            .unwrap()
            .with_base_url(url);

        let result = provider.embed("hello").await.unwrap();
        assert_eq!(result, vec![0.5, 0.6]);
        mock.assert_async().await;
    }

    #[cfg(feature = "embed-voyage")]
    #[tokio::test]
    async fn test_voyage_embed_batch_mock() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let mock = server
            .mock("POST", "/embeddings")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "data": [
                        { "embedding": [0.7, 0.8] },
                        { "embedding": [0.9, 1.0] }
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let provider = VoyageProvider::new("test-key".into())
            .unwrap()
            .with_base_url(url);

        let result = provider.embed_batch(&["x", "y"]).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec![0.7, 0.8]);
        assert_eq!(result[1], vec![0.9, 1.0]);
        mock.assert_async().await;
    }

    #[cfg(feature = "embed-openai")]
    #[tokio::test]
    async fn test_remote_provider_error_handling() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let _mock = server
            .mock("POST", "/embeddings")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let provider = OpenAiProvider::new("wrong-key".into())
            .unwrap()
            .with_base_url(url);

        let result = provider.embed("hello").await;
        assert!(result.is_err());
    }
}
