// Retry only the model call. Previously executed Agent tools are not replayed.
async fn web_generate_with_retry<P: AgentLlmProvider>(
    provider: &P,
    system: &str,
    user: &str,
    images: &[super::types::AgentImage],
    cancellation: Option<&AgentCancellationToken>,
) -> Result<String, String> {
    let first = generate_with_cancellation(provider, system, user, images, cancellation).await;
    match first {
        Err(error) if error == "LLM response did not contain assistant content"
            || (error.starts_with("LLM HTTP 500")
                && error.contains("Unexpected token")
                && error.contains("while expecting start token")) =>
        {
            generate_with_cancellation(provider, system, user, images, cancellation)
                .await
                .map_err(|retry| format!("{error}; model retry failed: {retry}"))
        }
        result => result,
    }
}
