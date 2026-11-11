//! Single-user loopback Web host, with an explicitly separate fixture mode.
use super::*;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::{sync_channel, SyncSender}};
use std::io::{Write, Read};

#[path="ui_bridge.rs"]
mod ui;
#[path="fixture_model.rs"] mod fixture_model;
#[path="model_relay.rs"] mod model_relay;
#[path="uploads.rs"] mod uploads;
#[path="downloads.rs"] mod downloads;

struct Probe {
    model_trace: Arc<Mutex<Vec<serde_json::Value>>>,
    watcher: Mutex<Option<commands::file_sync::ExternalWatcher>>,
    root: std::path::PathBuf,
    settings: Mutex<serde_json::Value>,
    ui_events: Mutex<std::collections::HashMap<String,Vec<SyncSender<String>>>>,
    project: Option<types::wiki::WikiProject>,
    fixture_model: Option<agent::provider::LlmConfig>,
    sessions: agent::session::AgentSessionStore,
    cancels: agent::cancel::AgentCancellationRegistry,
    subscriber: Mutex<Option<SyncSender<String>>>,
    running: AtomicBool,
    relay_active: std::sync::atomic::AtomicUsize,
    cookie: String,
    ticket: Mutex<Option<String>>,
}

impl Probe {
    fn is_fixture(&self)->bool{self.fixture_model.is_some()}
    fn active_model(&self)->Result<agent::provider::LlmConfig,String>{
        let model=if let Some(model)=&self.fixture_model{model.clone()}else{
            serde_json::from_value(self.settings.lock().unwrap().get("llmConfig").cloned().unwrap_or(serde_json::Value::Null))
                .map_err(|_|"Configure a model in Settings before sending a request".to_string())?
        };
        agent::provider::external_http_requests(&model)?;
        Ok(model)
    }
}

fn reply(request: tiny_http::Request, code: u16, value: serde_json::Value) {
    let _ = request.respond(tiny_http::Response::from_string(value.to_string()).with_status_code(code)
        .with_header(tiny_http::Header::from_bytes("Content-Type","application/json").unwrap()));
}

fn write_settings(root:&std::path::Path,value:&serde_json::Value)->Result<(),String>{
    let file=root.join("external-ui-state.json");let tmp=root.join(format!(".external-ui-state-{}.tmp",Uuid::new_v4()));
    let mut options=std::fs::OpenOptions::new();options.write(true).create_new(true);
    #[cfg(unix)]{use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
    let mut handle=options.open(&tmp).map_err(|e|e.to_string())?;
    let result=(||{handle.write_all(&serde_json::to_vec_pretty(value).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;handle.sync_all().map_err(|e|e.to_string())?;std::fs::rename(&tmp,&file).map_err(|e|e.to_string())})();
    if result.is_err(){let _=std::fs::remove_file(tmp);}result
}

pub fn serve() -> Result<(), String> {
    let mode=std::env::var("EXTERNAL_HOST_MODEL_MODE").unwrap_or_else(|_|"fixture".into());
    if mode!="fixture"&&mode!="live"{return Err("Model mode must be fixture or live".into());}
    let port=std::env::var("EXTERNAL_HOST_PROBE_PORT").unwrap_or_else(|_|"18902".into()).parse::<u16>().map_err(|e|e.to_string())?;
    let server=tiny_http::Server::http(format!("127.0.0.1:{port}")).map_err(|e|e.to_string())?;
    let root=std::path::PathBuf::from(std::env::var("EXTERNAL_HOST_FIXTURE_ROOT").map_err(|e|e.to_string())?).canonicalize().map_err(|e|e.to_string())?;
    let saved=match std::fs::read_to_string(root.join("external-ui-state.json")){
        Ok(raw)=>{let value:serde_json::Value=serde_json::from_str(&raw).map_err(|e|format!("Invalid saved settings: {e}"))?;if !value.is_object(){return Err("Saved settings must be an object".into());}Some(value)},
        Err(error) if error.kind()==std::io::ErrorKind::NotFound=>None,
        Err(error)=>return Err(error.to_string()),
    };
    let project=if let Some(settings)=&saved {
        if settings.get("lastProject").map(|p|p.is_null()).unwrap_or(true){None}else{
        let path=settings["lastProject"]["path"].as_str().ok_or("Saved project path missing; refusing to replace saved settings")?;
        let candidate=std::path::Path::new(path);
        if candidate.components().any(|c|matches!(c,std::path::Component::ParentDir))||!candidate.canonicalize().map_err(|e|e.to_string())?.starts_with(&root){return Err("Saved project is outside fixture root".into());}
        Some(commands::project::open_project(path.into())?)
        }
    }else{
        let name=if mode=="fixture"{format!("http-fixture-{}",Uuid::new_v4())}else{"Web Wiki".into()};
        let project=commands::project::create_project(name,root.to_string_lossy().into_owned())?;
        if mode=="fixture"{
            tauri::async_runtime::block_on(commands::fs::write_file(format!("{}/wiki/concepts/fixture.md",project.path),
                "---\ntitle: Fixture\ntype: concept\n---\n# Fixture\nThe fixture color is blue.\n".into()))?;
        }
        Some(project)
    };
    let (fixture_model,model_trace)=if mode=="fixture"{
        let (port,trace)=fixture_model::start()?;
        let config:agent::provider::LlmConfig=serde_json::from_value(serde_json::json!({"provider":"custom","model":"fixture","customEndpoint":format!("http://127.0.0.1:{port}/v1"),"streamingEnabled":true,"maxTokens":100})).map_err(|e|e.to_string())?;
        (Some(config),trace)
    }else{(None,Arc::new(Mutex::new(Vec::new())))};
    let mut settings=saved.unwrap_or_else(||{let project=project.as_ref().expect("new workspace project");serde_json::json!({"language":"en","lastProject":{"id":"fixture","name":project.name,"path":project.path},"projectRegistry":{"fixture":{"id":"fixture","name":project.name,"path":project.path}},
        "llmConfig":{"provider":"custom","apiKey":"","model":"","ollamaUrl":"http://127.0.0.1:11434","customEndpoint":"","maxContextSize":50000,"streamingEnabled":true},
        "generalConfig":{"autostart":false,"closeBehavior":"exit"},"updateCheckState":{"enabled":false},"sourceWatchConfig":{"enabled":false,"autoIngest":false}})});
    if let Some(model)=&fixture_model{
        // The fixture owns this volatile port. Live configurations are never
        // rewritten on startup, and a missing live model never falls back.
        settings["llmConfig"]=serde_json::json!({"provider":"custom","apiKey":"","model":"fixture","ollamaUrl":"","customEndpoint":model.custom_endpoint,"maxContextSize":50000,"streamingEnabled":true});
    }
    write_settings(&root,&settings)?;
    let ticket=Uuid::new_v4().to_string();
    let state=Arc::new(Probe{model_trace,watcher:Mutex::new(None),root,settings:Mutex::new(settings),ui_events:Mutex::new(Default::default()),project,fixture_model,sessions:Default::default(),cancels:Default::default(),subscriber:Mutex::new(None),running:AtomicBool::new(false),relay_active:std::sync::atomic::AtomicUsize::new(0),cookie:Uuid::new_v4().to_string(),ticket:Mutex::new(Some(ticket.clone()))});

    let label=if state.is_fixture(){"LLM Wiki fixture"}else{"LLM Wiki Web"};
    println!("{label}: http://localhost:{port}/login?ticket={ticket}");
    for mut request in server.incoming_requests() {
        let host=request.headers().iter().find(|h|h.field.equiv("Host")).map(|h|h.value.as_str()).unwrap_or("");
        if host!=format!("localhost:{port}") && host!=format!("127.0.0.1:{port}"){reply(request,403,serde_json::json!({"error":"Host rejected"}));continue;}
        if request.method()==&tiny_http::Method::Post {
            if let Some(origin)=request.headers().iter().find(|h|h.field.equiv("Origin")) {
                if origin.value.as_str()!=format!("http://localhost:{port}") && origin.value.as_str()!=format!("http://127.0.0.1:{port}") {reply(request,403,serde_json::json!({"error":"Origin rejected"}));continue;}
            }
        }
        let url=request.url().to_string();
        if request.method()==&tiny_http::Method::Get && url.starts_with("/login?ticket=") {
            let valid={let mut saved=state.ticket.lock().unwrap();if saved.as_deref()==url.strip_prefix("/login?ticket="){saved.take();true}else{false}};
            if !valid{reply(request,401,serde_json::json!({"error":"Invalid or used ticket"}));continue;}
            let _=request.respond(tiny_http::Response::empty(303).with_header(tiny_http::Header::from_bytes("Location","/").unwrap())
                .with_header(tiny_http::Header::from_bytes("Set-Cookie",format!("external_probe={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=3600",state.cookie)).unwrap()));
            continue;
        }
        let authenticated=request.headers().iter().filter(|h|h.field.equiv("Cookie")).any(|h|h.value.as_str().split(';').any(|v|v.trim()==format!("external_probe={}",state.cookie)));
        if !authenticated{reply(request,401,serde_json::json!({"error":"Login required"}));continue;}
        if request.method()==&tiny_http::Method::Post && url=="/login-ticket" {
            let ticket=Uuid::new_v4().to_string();*state.ticket.lock().unwrap()=Some(ticket.clone());
            reply(request,200,serde_json::json!({"url":format!("http://localhost:{port}/login?ticket={ticket}")}));continue;
        }
        if request.method()==&tiny_http::Method::Post && url=="/_llmwiki/invoke" {
            let mut body=String::new();
            if std::io::Read::read_to_string(&mut request.as_reader().take(2_097_153),&mut body).is_err() || body.len()>2_097_152 {reply(request,400,serde_json::json!({"ok":false,"error":"Invalid or oversized invocation"}));continue;}
            let result=(||{let value:serde_json::Value=serde_json::from_str(&body).map_err(|e|e.to_string())?;ui::invoke(&state,value["command"].as_str().ok_or("Command missing")?,&value["args"])})();
            match result{Ok(value)=>reply(request,200,serde_json::json!({"ok":true,"value":value})),Err(error)=>reply(request,400,serde_json::json!({"ok":false,"error":error}))};continue;
        }
        if request.method()==&tiny_http::Method::Get && url.starts_with("/_llmwiki/events?") {
            let parsed=reqwest::Url::parse(&format!("http://probe{url}")).unwrap();
            let name=parsed.query_pairs().find(|(key,_)|key=="event").map(|(_,value)|value.into_owned()).unwrap_or_default();ui::events(request,state.clone(),name);continue;
        }
        if request.method()==&tiny_http::Method::Post && url=="/_llmwiki/http" {let shared=state.clone();std::thread::spawn(move||model_relay::relay(request,&shared));continue;}
        if request.method()==&tiny_http::Method::Get && url.starts_with("/_llmwiki/asset?") {downloads::serve(request,&state.root);continue;}
        if request.method()==&tiny_http::Method::Post && url=="/_llmwiki/upload" {let shared=state.clone();std::thread::spawn(move||uploads::receive(request,&shared.root));continue;}
        if state.is_fixture()&&request.method()==&tiny_http::Method::Get && url=="/probe-trace" {reply(request,200,serde_json::json!({"calls":*state.model_trace.lock().unwrap()}));continue;}
        if url.starts_with("/_llmwiki/"){reply(request,501,serde_json::json!({"error":"Not yet supported by unchanged-source experiment"}));continue;}
        if request.method()==&tiny_http::Method::Get && (url=="/" || url.starts_with("/assets/") || url=="/vite.svg") {ui::static_file(request,&url);continue;}
        if !state.is_fixture(){reply(request,404,serde_json::json!({"error":"Unknown Web route"}));continue;}
        match (request.method().as_str(),url.as_str()) {
            ("GET","/probe")=>{let _=request.respond(tiny_http::Response::from_string(include_str!("probe.html")).with_header(tiny_http::Header::from_bytes("Content-Type","text/html; charset=utf-8").unwrap()).with_header(tiny_http::Header::from_bytes("Cache-Control","no-store").unwrap()));}
            ("GET","/session")=>reply(request,200,serde_json::json!({"messages":state.sessions.recent_messages(&state.project.as_ref().expect("fixture project").path,"http-probe",10)})),
            ("GET","/events")=>{
                let (sender,receiver)=sync_channel::<String>(128);
                let mut subscriber=state.subscriber.lock().unwrap();
                if subscriber.is_some(){reply(request,409,serde_json::json!({"error":"Only one probe subscriber supported"}));continue;}
                *subscriber=Some(sender);drop(subscriber);
                let shared=state.clone();
                std::thread::spawn(move || {
                    let mut writer=request.into_writer();
                    let mut alive=writer.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n: ready\n\n").and_then(|_|writer.flush()).is_ok();
                    while alive {
                        let frame=match receiver.recv_timeout(std::time::Duration::from_secs(2)){Ok(v)=>v,Err(std::sync::mpsc::RecvTimeoutError::Timeout)=>": heartbeat\n\n".into(),Err(_)=>break};
                        alive=writer.write_all(frame.as_bytes()).and_then(|_|writer.flush()).is_ok();
                    }
                    shared.subscriber.lock().unwrap().take();
                    shared.cancels.cancel(&shared.project.as_ref().expect("fixture project").name,"http-probe",None);
                });
            }
            ("POST","/run")=>{
                let sender=state.subscriber.lock().unwrap().clone();
                if sender.is_none(){reply(request,409,serde_json::json!({"error":"Subscribe before running"}));continue;}
                if state.running.swap(true,Ordering::SeqCst){reply(request,409,serde_json::json!({"error":"Probe already running"}));continue;}
                let shared=state.clone();let sender=sender.unwrap();
                std::thread::spawn(move || {
                    let run_id=Uuid::new_v4().to_string();
                    let input:agent::AgentChatRequest=serde_json::from_value(serde_json::json!({"message":"What color is the fixture?","sessionId":"http-probe","runId":run_id,"mode":"fast"})).unwrap();
                    let user=input.message.clone();
                    let token=shared.cancels.start(&shared.project.as_ref().expect("fixture project").name,"http-probe",&run_id);
                    let output=sender.clone();let cancels=shared.cancels.clone();let project_id=shared.project.as_ref().expect("fixture project").name.clone();let active_run=run_id.clone();
                    let sink:agent::runtime::AgentEventSink=Arc::new(move |mut event|{
                        event.redact_for_external_api();
                        if output.try_send(format!("data: {}\n\n",serde_json::to_string(&event).unwrap())).is_err(){cancels.cancel(&project_id,"http-probe",Some(&active_run));}
                    });
                    let runtime=agent::AgentRuntime::new(&shared.project.as_ref().expect("fixture project").name,&shared.project.as_ref().expect("fixture project").path,None,Some(shared.active_model().expect("fixture configuration")),None,None);
                    let result=tauri::async_runtime::block_on(runtime.run_once_with_cancel_and_events(input,Some(token),Some(sink)));
                    shared.cancels.finish(&shared.project.as_ref().expect("fixture project").name,"http-probe",&run_id);
                    let final_value=match result {Ok(response)=>{shared.sessions.append_turn(&shared.project.as_ref().expect("fixture project").path,&shared.project.as_ref().expect("fixture project").name,&response.session_id,&user,&response.message);serde_json::json!({"ok":true,"message":response.message})},Err(e)=>serde_json::json!({"ok":false,"error":e})};
                    let _=sender.try_send(format!("event: complete\ndata: {final_value}\n\n"));
                    shared.running.store(false,Ordering::SeqCst);
                });
                reply(request,202,serde_json::json!({"started":true}));
            }
            _=>reply(request,404,serde_json::json!({"error":"Unknown probe route"})),
        }
    }
    Ok(())
}
