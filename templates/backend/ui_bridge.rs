use super::*;
use serde_json::{Value,json};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path,Component};
use std::sync::mpsc::sync_channel;

fn argument<T:DeserializeOwned>(args:&Value,key:&str)->Result<T,String>{serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null)).map_err(|e|format!("Invalid {key}: {e}"))}
fn encode<T:Serialize>(value:T)->Result<Value,String>{serde_json::to_value(value).map_err(|e|e.to_string())}
include!("pure_commands.rs");
async fn dispatch_runtime(command:&str,_args:&Value)->Result<Value,String>{Err(format!("Unsupported in unchanged-source experiment: {command}; desktop runtime integration pending"))}
fn checked(state:&Probe,path:&str)->Result<std::path::PathBuf,String>{
    let candidate=Path::new(path);
    if !candidate.is_absolute() || candidate.components().any(|c|matches!(c,Component::ParentDir)){return Err("Absolute fixture path without traversal required".into());}
    let mut ancestor=candidate;
    while !ancestor.exists(){if ancestor.symlink_metadata().is_ok(){return Err("Unresolved symlink".into());}ancestor=ancestor.parent().ok_or("Invalid path")?;}
    if !ancestor.canonicalize().map_err(|e|e.to_string())?.starts_with(&state.root){return Err("Outside fixture workspace".into());}
    Ok(candidate.to_path_buf())
}
fn project(state:&Probe,args:&Value)->Result<(String,String),String>{
    let id:String=argument(args,"projectId")?;
    let settings=state.settings.lock().unwrap();
    let item=if id=="current"{settings.get("lastProject")}else{settings.get("projectRegistry").and_then(|r|r.get(&id))}.ok_or("Unknown project")?;
    let path=item["path"].as_str().ok_or("Project path missing")?.to_string();checked(state,&path)?;
    Ok((item["id"].as_str().unwrap_or(&id).to_string(),path))
}
pub(super) fn invoke(state:&Arc<Probe>,command:&str,args:&Value)->Result<Value,String>{
    if command=="_web_info"{return Ok(json!({"root":state.root,"mode":"unchanged-source-experiment","modelMode":if state.is_fixture(){"fixture"}else{"live"}}));}
    if command=="_web_store"{
        if args["store"]!="app-state.json"{return Err("Unsupported store".into());}
        let mut settings=state.settings.lock().unwrap();let key=args["key"].as_str().unwrap_or("");
        if args["action"]=="get"{return Ok(settings.get(key).cloned().unwrap_or(Value::Null));}
        if args["action"]=="flush"{return Ok(Value::Null);}
        let mut updated=settings.clone();
        let result=match args["action"].as_str(){Some("set")=>{updated[key]=args["value"].clone();Value::Null},Some("delete")=>json!(updated.as_object_mut().unwrap().remove(key).is_some()),_=>return Err("Unsupported store action".into())};
        write_settings(&state.root,&updated)?;*settings=updated;return Ok(result);
    }
    for key in ["path","projectPath","filePath","source","destination","archivePath","sourcePath","destDir","relTo","workingDirectory"]{
        if let Some(path)=args[key].as_str(){if key!="filePath"||Path::new(path).is_absolute(){checked(state,path)?;}else if Path::new(path).components().any(|c|matches!(c,Component::ParentDir)){return Err("Traversal rejected".into());}}
    }
    if command=="create_project"{let name:String=argument(args,"name")?;if name.is_empty()||name=="."||name==".."||name.contains(['/', '\\']){return Err("Invalid project name".into());}}
    match command{
        "start_project_file_watcher"=>{
            let id:String=argument(args,"projectId")?;let path:String=argument(args,"projectPath")?;
            let config:Option<commands::file_sync::SourceWatchConfig>=argument(args,"sourceWatchConfig")?;
            let config=config.unwrap_or_default();
            let weak=Arc::downgrade(state);
            let sink:commands::file_sync::ExternalSink=Arc::new(move|name,value|{if let Some(state)=weak.upgrade(){emit(&state,name,value);}});
            let mut slot=state.watcher.lock().unwrap();slot.take();
            let watcher=commands::file_sync::external_watch(std::path::PathBuf::from(&path),id.clone(),config.clone(),sink.clone())?;
            *slot=Some(watcher);
            encode(commands::file_sync::external_rescan(Path::new(&path),&id,&config,&sink)?)
        }
        "stop_project_file_watcher"=>{state.watcher.lock().unwrap().take();Ok(Value::Null)}
        "rescan_project_files"=>{
            let id:String=argument(args,"projectId")?;let path:String=argument(args,"projectPath")?;
            let config:Option<commands::file_sync::SourceWatchConfig>=argument(args,"sourceWatchConfig")?;
            let config=config.unwrap_or_default();let weak=Arc::downgrade(state);
            let sink:commands::file_sync::ExternalSink=Arc::new(move|name,value|{if let Some(state)=weak.upgrade(){emit(&state,name,value);}});
            encode(commands::file_sync::external_rescan(Path::new(&path),&id,&config,&sink)?)
        }
        "agent_list_skills"=>encode(agent::skills::agent_list_skills(argument(args,"projectPath")?)),
        "agent_list_sessions"|"agent_get_session"|"agent_cancel_turn"=>{
            let(id,path)=project(state,args)?;
            if command=="agent_list_sessions"{return encode(state.sessions.list_sessions(&path));}
            let session:String=argument(args,"sessionId")?;
            if command=="agent_get_session"{return encode(state.sessions.recent_messages(&path,&session,40));}
            let run:Option<String>=argument(args,"runId")?;encode(state.cancels.cancel(&id,&session,run.as_deref()))
        }
        "agent_start_turn_stream"=>{
            let(id,path)=project(state,args)?;let mut input:agent::AgentChatRequest=argument(args,"request")?;
            let model=state.active_model()?;
            if state.running.swap(true,Ordering::SeqCst){return Err("Another Agent turn is still running".into());}
            let session=input.session_id.clone().unwrap_or_else(||Uuid::new_v4().to_string());let run=input.run_id.clone().unwrap_or_else(||Uuid::new_v4().to_string());input.session_id=Some(session.clone());input.run_id=Some(run.clone());
            if input.history.is_empty()&&!input.history_explicit{input.history=state.sessions.recent_messages(&path,&session,12).into_iter().map(|m|agent::types::AgentConversationMessage{role:m.role,content:m.content}).collect();}
            let started_session=session.clone();let shared=state.clone();
            std::thread::spawn(move||{
                let user=input.message.clone();let persist=input.persist_session;let token=shared.cancels.start(&id,&session,&run);let output=shared.clone();let s=session.clone();let r=run.clone();
                let sink:agent::runtime::AgentEventSink=Arc::new(move|event|emit(&output,"agent-event",json!({"sessionId":s,"runId":r,"event":event})));
                let runtime=agent::AgentRuntime::new(&id,&path,None,Some(model),None,None);
                let result=tauri::async_runtime::block_on(runtime.run_once_with_cancel_and_events(input,Some(token),Some(sink)));
                shared.cancels.finish(&id,&session,&run);
                match result{Ok(response)=>{if persist{shared.sessions.append_turn(&path,&id,&session,&user,&response.message);}},Err(error)=>emit(&shared,"agent-event",json!({"sessionId":session,"runId":run,"event":{"type":"error","message":error}}))}
                shared.running.store(false,Ordering::SeqCst);
            });Ok(json!(started_session))
        }
        "set_close_behavior" if args["value"]=="exit"=>Ok(Value::Null),
        _=>tauri::async_runtime::block_on(dispatch_command(command,args)),
    }
}
fn emit(state:&Probe,name:&str,value:Value){
    if let Some(list)=state.ui_events.lock().unwrap().get_mut(name){list.retain(|sender|sender.try_send(format!("data: {value}\n\n")).is_ok());}
}
pub(super) fn events(request:tiny_http::Request,state:Arc<Probe>,name:String){
    let (sender,receiver)=sync_channel::<String>(128);
    let mut all=state.ui_events.lock().unwrap();
    if all.values().map(Vec::len).sum::<usize>()>=32{reply(request,429,json!({"error":"Too many listeners"}));return;}
    all.entry(name).or_default().push(sender);drop(all);
    std::thread::spawn(move||{let mut writer=request.into_writer();if writer.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n: ready\n\n").and_then(|_|writer.flush()).is_err(){return;}
        loop{let frame=match receiver.recv_timeout(std::time::Duration::from_secs(2)){Ok(v)=>v,Err(std::sync::mpsc::RecvTimeoutError::Timeout)=>": heartbeat\n\n".into(),Err(_)=>break};if writer.write_all(frame.as_bytes()).and_then(|_|writer.flush()).is_err(){break;}}
    });
}
pub(super) fn static_file(request:tiny_http::Request,url:&str){
    let root=std::env::var_os("EXTERNAL_HOST_STATIC_ROOT").map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dist"));let relative=url.trim_start_matches('/');
    if Path::new(relative).components().any(|c|!matches!(c,Component::Normal(_))){reply(request,403,json!({"error":"Invalid resource path"}));return;}
    let file=if relative.is_empty(){root.join("index.html")}else{root.join(relative)};
    let ext=file.extension().and_then(|v|v.to_str()).unwrap_or("");
    let kind=match ext{"html"=>"text/html; charset=utf-8","js"=>"text/javascript","css"=>"text/css","svg"=>"image/svg+xml","png"=>"image/png","woff2"=>"font/woff2",_=>"application/octet-stream"};
    match std::fs::File::open(file){Ok(file)=>{let _=request.respond(tiny_http::Response::from_file(file).with_header(tiny_http::Header::from_bytes("Content-Type",kind).unwrap()));},Err(_)=>reply(request,404,json!({"error":"Resource missing"}))}
}
