// External host: compile the original crate root without editing its source.
include!(concat!(env!("OUT_DIR"), "/host_module_graph.rs"));
use std::sync::Mutex;
use uuid::Uuid;

mod http_probe;

fn main() -> Result<(), String> {
    if std::env::var("EXTERNAL_HOST_HTTP_PROBE").as_deref()==Ok("1") {return http_probe::serve();}
    let root = std::env::var("EXTERNAL_HOST_FIXTURE_ROOT").map_err(|e| e.to_string())?;
    let project = commands::project::create_project(format!("external-host-fixture-{}", Uuid::new_v4()), root)?;
    let path = format!("{}/wiki/external-host-fixture.md", project.path);
    tauri::async_runtime::block_on(commands::fs::write_file(path.clone(), "Synthetic unchanged-source host fixture".into()))?;
    let text = tauri::async_runtime::block_on(commands::fs::read_file(path, Some(false)))?;
    assert_eq!(text, "Synthetic unchanged-source host fixture");
    let reopened = commands::project::open_project(project.path.clone())?;
    assert_eq!(reopened.path, project.path);
    println!("PASS: original create_project, write_file, read_file and open_project through external host");
    agent_smoke(&project)?;
    Ok(())
}

fn agent_smoke(project: &types::wiki::WikiProject) -> Result<(), String> {
    let fixture = format!("{}/wiki/concepts/fixture.md", project.path);
    tauri::async_runtime::block_on(commands::fs::write_file(fixture,
        "---\ntitle: Fixture\ntype: concept\n---\n# Fixture\nThe fixture color is blue.\n".into()))?;
    let server = tiny_http::Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = server.server_addr().to_ip().unwrap().port();
    let worker = std::thread::spawn(move || {
        for index in 0..2 {
            let mut request = server.recv_timeout(std::time::Duration::from_secs(30)).unwrap().expect("mock model request missing");
            let mut body = String::new();
            std::io::Read::read_to_string(request.as_reader(), &mut body).unwrap();
            let input: serde_json::Value = serde_json::from_str(&body).unwrap();
            let action = if index == 0 {
                serde_json::json!({"action":"tool","tool":"wiki.read_page","path":"wiki/concepts/fixture.md"})
            } else {
                serde_json::json!({"action":"final","answer":"Fixture color is blue."})
            }.to_string();
            let (content, content_type) = if input["stream"] == true {
                (format!("data: {}\n\ndata: [DONE]\n\n", serde_json::json!({"choices":[{"delta":{"content":action}}]})), "text/event-stream")
            } else {
                (serde_json::json!({"choices":[{"message":{"role":"assistant","content":action},"finish_reason":"stop"}]}).to_string(), "application/json")
            };
            request.respond(tiny_http::Response::from_string(content).with_header(
                tiny_http::Header::from_bytes("Content-Type", content_type).unwrap())).unwrap();
        }
    });
    let config = serde_json::from_value(serde_json::json!({"provider":"custom","model":"fixture",
        "customEndpoint":format!("http://127.0.0.1:{port}/v1"),"maxTokens":100,"streamingEnabled":true})).map_err(|e| e.to_string())?;
    let runtime = agent::AgentRuntime::new(&project.name, &project.path, None, Some(config), None, None);
    let request: agent::AgentChatRequest = serde_json::from_value(serde_json::json!({
        "message":"What color is the fixture?","sessionId":"external-session","runId":"fixture-run","mode":"fast"
    })).map_err(|e|e.to_string())?;
    let events = std::sync::Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let sink: agent::runtime::AgentEventSink = std::sync::Arc::new(move |event| captured.lock().unwrap().push(event));
    let registry = agent::cancel::AgentCancellationRegistry::default();
    let token = registry.start(&project.name, "external-session", "fixture-run");
    let response = tauri::async_runtime::block_on(runtime.run_once_with_cancel_and_events(request.clone(),Some(token),Some(sink)))?;
    registry.finish(&project.name,"external-session","fixture-run");
    worker.join().map_err(|_|"mock model worker panicked")?;
    assert_eq!(response.message,"Fixture color is blue.");
    let recorded = events.lock().unwrap();
    assert!(recorded.iter().any(|e|matches!(e,agent::events::AgentEvent::ToolEnd{tool,..} if tool=="wiki.read_page")));
    assert!(recorded.iter().any(|e|matches!(e,agent::events::AgentEvent::Done{..})));
    assert!(recorded.iter().any(|e|matches!(e,agent::events::AgentEvent::MessageDelta{text} if text.contains("blue"))));
    let sessions = agent::session::AgentSessionStore::default();
    sessions.append_turn(&project.path,&project.name,&response.session_id,&request.message,&response.message);
    let restored = agent::session::AgentSessionStore::default();
    assert_eq!(restored.recent_messages(&project.path,"external-session",10).last().unwrap().content,"Fixture color is blue.");
    let token = registry.start(&project.name,"external-session","cancel-fixture");
    assert!(registry.cancel(&project.name,"external-session",Some("cancel-fixture")));
    let error = tauri::async_runtime::block_on(runtime.run_once_with_cancel(request,Some(token))).unwrap_err();
    assert!(error.to_lowercase().contains("cancel"));
    registry.finish(&project.name,"external-session","cancel-fixture");
    println!("PASS: unchanged Agent tool execution, stream callbacks, persisted session reload and cancellation");
    Ok(())
}
