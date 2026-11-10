//! Deterministic synthetic protocol fixture. Never a real model provider.
use super::*;
pub fn start()->Result<(u16,Arc<Mutex<Vec<serde_json::Value>>>),String>{
    let server=tiny_http::Server::http("127.0.0.1:0").map_err(|e|e.to_string())?;
    let port=server.server_addr().to_ip().unwrap().port();let trace=Arc::new(Mutex::new(Vec::new()));let output=trace.clone();
    std::thread::spawn(move||{
        let mut generated_page=String::new();
        for mut request in server.incoming_requests(){
            let mut body=String::new();if std::io::Read::read_to_string(&mut request.as_reader().take(1_048_577),&mut body).is_err()||body.len()>1_048_576{continue;}
            let input:serde_json::Value=serde_json::from_str(&body).unwrap();
            let system=input["messages"][0]["content"].as_str().unwrap_or("");
            let user=input["messages"].as_array().and_then(|m|m.last()).and_then(|m|m["content"].as_str()).unwrap_or("");
            let(stage,answer)=if user.starts_with("Analyze this source document:"){
                ("analysis","## Synthetic source analysis\nThe source defines a test concept and states that its signal color is cobalt. Create one source summary and one concept page; retain source attribution.".to_string())
            }else if system.contains("Based on the analysis provided, generate wiki files"){
                let summary=system.split("A source summary page at **").nth(1).and_then(|s|s.split("**").next()).expect("fixture summary path missing");
                let source=user.split("Source document to process: **").nth(1).and_then(|s|s.split("**").next()).expect("fixture source identity missing");
                let stem=source.trim_end_matches(".txt").rsplit('/').next().unwrap();
                generated_page=format!("wiki/concepts/{stem}-concept.md");
                let title=format!("Synthetic Import Verification {}",stem.trim_start_matches("ingest-proof-"));
                let source_json=serde_json::to_string(source).unwrap();
                ("generation",format!("---FILE: {generated_page}---\n---\ntitle: {title}\ntype: concept\nsources: [{source_json}]\n---\n# {title}\nThe synthetic signal color is cobalt. Related test page: [[Fixture]].\n---END FILE---\n\n---FILE: {summary}---\n---\ntitle: Source {stem}\ntype: source\nsources: [{source_json}]\n---\n# Source {stem}\nThe synthetic source states that the signal color is cobalt. See [[{title}]].\n---END FILE---\n\n---FILE: wiki/log.md---\n## [{}] ingest | {title}\nImported synthetic fixture {source}.\n---END FILE---",chrono::Utc::now().format("%Y-%m-%d")))
            }else if system.contains("Agent loop protocol:"){
                let imported=user.contains("What is the synthetic signal color?");
                let first=user.contains("Tool observations so far: none.");
                let path=if imported&&!generated_page.is_empty(){generated_page.as_str()}else{"wiki/concepts/fixture.md"};
                let action=if first{serde_json::json!({"action":"tool","tool":"wiki.read_page","path":path})}
                    else{serde_json::json!({"action":"final","answer":if imported{"The synthetic signal color is cobalt."}else{"Fixture color is blue."}})};
                ("agent",action.to_string())
            }else{
                output.lock().unwrap().push(serde_json::json!({"stage":"unsupported"}));
                let _=request.respond(tiny_http::Response::from_string("Unsupported fixture protocol").with_status_code(400));continue;
            };
            output.lock().unwrap().push(serde_json::json!({"stage":stage,"stream":input["stream"]==true}));
            let(content,kind)=if input["stream"]==true{
                let chunks=answer.chars().collect::<Vec<_>>().chunks(96).map(|chunk|format!("data: {}\n\n",serde_json::json!({"choices":[{"delta":{"content":chunk.iter().collect::<String>()}}]}))).collect::<String>();
                (format!("{chunks}data: [DONE]\n\n"),"text/event-stream")
            }else{(serde_json::json!({"choices":[{"message":{"role":"assistant","content":answer},"finish_reason":"stop"}]}).to_string(),"application/json")};
            let _=request.respond(tiny_http::Response::from_string(content).with_header(tiny_http::Header::from_bytes("Content-Type",kind).unwrap()));
        }
    });
    Ok((port,trace))
}
