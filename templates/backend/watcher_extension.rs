// External host additions. All scanning, queue, snapshot and filter functions
// below come from the unchanged original file_sync.rs included before this file.
pub type ExternalSink = Arc<dyn Fn(&str,serde_json::Value)+Send+Sync>;

pub fn external_rescan(root:&Path,project_id:&str,config:&SourceWatchConfig,sink:&ExternalSink)->Result<FileChangeRescanResult,String>{
    ensure_sync_dir(root)?;
    // Synchronize app-owned writes before reconciliation, using the original
    // suppression registry and snapshot writer, so they are not re-ingested.
    let rules=SourceWatchRules::new(config);
    let mut own_writes=BTreeSet::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok){
        if entry.file_type().is_file()&&is_app_write_ignored(entry.path()){
            if let Some(rel)=relative_watch_path(root,entry.path(),&rules,entry.metadata().ok().map(|m|m.len())){own_writes.insert(rel);}
        }
    }
    if !own_writes.is_empty(){sync_snapshot_paths(root,own_writes)?;}
    enqueue_rescan_changes(root,project_id,config)?;
    let changed_tasks=process_queue_inner(root,project_id,
        |queue|sink(EVENT_QUEUE_UPDATED,serde_json::to_value(FileSyncPayload{project_id:project_id.to_string(),tasks:queue.tasks.clone()}).unwrap()),
        |tasks|{if !tasks.is_empty(){sink(EVENT_CHANGED,serde_json::to_value(FileSyncPayload{project_id:project_id.to_string(),tasks}).unwrap());}})?;
    let queue=with_queue_lock(root,||read_queue(root))?;
    Ok(FileChangeRescanResult{queue,changed_tasks})
}

pub struct ExternalWatcher {
    watcher: Option<RecommendedWatcher>,
    root: PathBuf,
    active: Arc<std::sync::atomic::AtomicBool>,
    wake: mpsc::SyncSender<Vec<PathBuf>>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Drop for ExternalWatcher {
    fn drop(&mut self) {
        self.active.store(false,Ordering::Release);
        self.watcher.take();
        let _=self.wake.try_send(Vec::new());
        if let Some(worker)=self.worker.take(){let _=worker.join();}
    }
}
impl ExternalWatcher {
    // Fault injection for the isolated reconciliation test, not a UI command.
    pub fn pause_native_for_probe(&mut self)->Result<(),String>{
        self.watcher.as_mut().ok_or("Watcher stopped")?.unwatch(&self.root).map_err(|e|e.to_string())
    }
}

pub fn external_watch(root:PathBuf,project_id:String,config:SourceWatchConfig,sink:ExternalSink)->Result<ExternalWatcher,String>{
    let interval=if cfg!(target_os="linux"){LINUX_WATCHER_RESCAN_INTERVAL_MS}else{OTHER_WATCHER_RESCAN_INTERVAL_MS};
    external_watch_with_period(root,project_id,config,sink,Duration::from_millis(interval as u64))
}
pub fn external_watch_with_period(root:PathBuf,project_id:String,config:SourceWatchConfig,sink:ExternalSink,period:Duration)->Result<ExternalWatcher,String>{
    let config=normalize_source_watch_config(Some(config));
    ensure_sync_dir(&root)?;
    with_queue_lock(&root,||reset_processing_tasks(&root,&project_id))?;
    external_rescan(&root,&project_id,&config,&sink)?;
    let active=Arc::new(std::sync::atomic::AtomicBool::new(true));
    let (sender,receiver)=mpsc::sync_channel::<Vec<PathBuf>>(8192);
    let callback_sender=sender.clone();let callback_active=active.clone();
    let mut watcher=notify::recommended_watcher(move|result:Result<Event,notify::Error>|{
        if !callback_active.load(Ordering::Acquire){return;}
        if let Ok(event)=result{if is_content_change_event(&event.kind){let _=callback_sender.try_send(event.paths);}}
    }).map_err(|e|e.to_string())?;
    watcher.watch(&root,RecursiveMode::Recursive).map_err(|e|e.to_string())?;
    let worker_root=root.clone();let worker_active=active.clone();
    let worker=std::thread::spawn(move||{
        let mut pending=false;let mut last_change=std::time::Instant::now();let mut last_scan=std::time::Instant::now();
        while worker_active.load(Ordering::Acquire){
            match receiver.recv_timeout(Duration::from_millis(100)){
                Ok(paths)=>{
                    let rules=SourceWatchRules::new(&config);
                    let snapshot=with_queue_lock(&worker_root,||read_snapshot(&worker_root)).unwrap_or_default();
                    let interesting=paths.iter().any(|path|{
                        if relative_watch_path(&worker_root,path,&rules,None).is_some(){return true;}
                        let mut known=BTreeSet::new();collect_known_paths(&worker_root,path,&snapshot,&mut known,&rules);!known.is_empty()
                    });
                    if interesting{pending=true;last_change=std::time::Instant::now();}
                }
                Err(mpsc::RecvTimeoutError::Timeout)=>{},Err(_)=>break,
            }
            if !worker_active.load(Ordering::Acquire){break;}
            if (pending&&last_change.elapsed()>=Duration::from_millis(700))||last_scan.elapsed()>=period{
                if let Err(error)=external_rescan(&worker_root,&project_id,&config,&sink){sink("external-watcher-error",serde_json::json!({"error":error}));}
                pending=false;last_scan=std::time::Instant::now();
            }
        }
    });
    Ok(ExternalWatcher{watcher:Some(watcher),root,active,wake:sender,worker:Some(worker)})
}
