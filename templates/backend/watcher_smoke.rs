#[path="../upstream/src-tauri/src/panic_guard.rs"]
mod panic_guard;
mod original_sync {
    include!("../upstream/src-tauri/src/commands/file_sync.rs");
    include!("watcher_extension.rs");
}
use std::sync::{Arc,mpsc};
use std::time::{Duration,Instant};

fn expect(receiver:&mpsc::Receiver<(String,serde_json::Value)>,kind:&str)->Result<(),String>{
    let deadline=Instant::now()+Duration::from_secs(10);
    while Instant::now()<deadline {
        let (name,value)=receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())).map_err(|e|e.to_string())?;
        if name=="external-watcher-error"{return Err(value.to_string());}
        if name=="file-sync://changed" && value["tasks"].as_array().unwrap().iter().any(|task|task["path"]=="raw/sources/fixture.txt" && task["kind"]==kind){return Ok(());}
    }
    Err(format!("Missing {kind} event"))
}
fn main()->Result<(),String>{
    let base=std::env::var("EXTERNAL_HOST_FIXTURE_ROOT").map_err(|e|e.to_string())?;
    let root=std::path::Path::new(&base).join(format!("watcher-{}",uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("raw/sources")).map_err(|e|e.to_string())?;
    let config:original_sync::SourceWatchConfig=serde_json::from_value(serde_json::json!({"enabled":true,"autoIngest":false})).map_err(|e|e.to_string())?;
    let (sender,receiver)=mpsc::sync_channel(128);
    let sink:original_sync::ExternalSink=Arc::new(move|name,value|{let _=sender.try_send((name.to_string(),value));});
    let watcher=original_sync::external_watch(root.clone(),"fixture-project".into(),config,sink)?;
    let file=root.join("raw/sources/fixture.txt");
    std::fs::write(&file,"Synthetic watcher fixture").map_err(|e|e.to_string())?;
    expect(&receiver,"created")?;
    for index in 0..5 {
        std::fs::write(&file,format!("Synthetic watcher fixture burst edit {index}")).map_err(|e|e.to_string())?;
    }
    expect(&receiver,"modified")?;
    let deadline=Instant::now()+Duration::from_millis(900);
    while Instant::now()<deadline {
        if let Ok((name,value))=receiver.recv_timeout(Duration::from_millis(50)){
            if name=="file-sync://changed"{assert!(!value["tasks"].as_array().unwrap().iter().any(|task|task["path"]=="raw/sources/fixture.txt"&&task["kind"]=="modified"),"Burst edits emitted duplicate changes");}
        }
    }
    std::fs::remove_file(&file).map_err(|e|e.to_string())?;
    expect(&receiver,"deleted")?;
    let snapshot:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join(".llm-wiki/file-snapshot.json")).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    assert!(snapshot["files"].get("raw/sources/fixture.txt").is_none());
    drop(watcher);
    while receiver.try_recv().is_ok(){}
    std::fs::write(&file,"Written after watcher stopped").map_err(|e|e.to_string())?;
    assert!(receiver.recv_timeout(Duration::from_millis(600)).is_err());
    println!("PASS: native file create/modify/delete -> original queue processor and event payloads; snapshot cleanup; burst debounce; watcher stop");
    let (sender,receiver)=mpsc::sync_channel(128);
    let sink:original_sync::ExternalSink=Arc::new(move|name,value|{let _=sender.try_send((name.to_string(),value));});
    let config:original_sync::SourceWatchConfig=serde_json::from_value(serde_json::json!({"enabled":true,"autoIngest":false})).unwrap();
    let mut watcher=original_sync::external_watch_with_period(root.clone(),"fixture-project".into(),config,sink,Duration::from_millis(150))?;
    while receiver.try_recv().is_ok(){}
    let own=root.join("raw/sources/app-write.txt");
    original_sync::mark_app_write_path(&own);
    std::fs::write(&own,"Synthetic app-owned write").map_err(|e|e.to_string())?;
    original_sync::mark_app_write_path(&own);
    let deadline=Instant::now()+Duration::from_millis(850);
    while Instant::now()<deadline {
        if let Ok((name,value))=receiver.recv_timeout(Duration::from_millis(50)){
            if name=="file-sync://changed"{assert!(!value["tasks"].as_array().unwrap().iter().any(|task|task["path"]=="raw/sources/app-write.txt"));}
        }
    }
    let snapshot:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join(".llm-wiki/file-snapshot.json")).unwrap()).unwrap();
    assert_eq!(snapshot["files"]["raw/sources/app-write.txt"]["size"],25);
    while receiver.try_recv().is_ok(){}
    watcher.pause_native_for_probe()?;
    std::fs::write(&file,"Changed while native notifications are disconnected").map_err(|e|e.to_string())?;
    expect(&receiver,"modified")?;
    drop(watcher);
    println!("PASS: app-owned write suppressed and snapshotted; periodic rescan recovered a change with native notifications disconnected");
    Ok(())
}
