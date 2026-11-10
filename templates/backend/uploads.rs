//! Authenticated browser uploads, staged separately from project files.
use super::*;
use std::path::{Path,PathBuf};
use std::sync::atomic::{AtomicUsize,Ordering};
static ACTIVE:AtomicUsize=AtomicUsize::new(0);
const LIMIT:u64=100*1024*1024;
struct Slot;
impl Drop for Slot{fn drop(&mut self){ACTIVE.fetch_sub(1,Ordering::SeqCst);}}

fn filename(raw:&str)->Result<String,String>{
    let mut decoded=Vec::new();let bytes=raw.as_bytes();let mut i=0;
    while i<bytes.len(){
        if bytes[i]==b'%' {
            if i+2>=bytes.len(){return Err("Invalid filename encoding".into());}
            let value=std::str::from_utf8(&bytes[i+1..i+3]).ok().and_then(|v|u8::from_str_radix(v,16).ok()).ok_or("Invalid filename encoding")?;
            decoded.push(value);i+=3;
        }else{decoded.push(bytes[i]);i+=1;}
    }
    let name=String::from_utf8(decoded).map_err(|_|"Filename must be UTF-8")?;
    if name.is_empty()||name.len()>240||name=="."||name==".."||name.chars().any(|c|c.is_control()||c=='/'||c=='\\'){
        return Err("Invalid filename".into());
    }
    Ok(name)
}

pub fn receive(mut request:tiny_http::Request,root:&Path){
    if ACTIVE.fetch_update(Ordering::SeqCst,Ordering::SeqCst,|n|if n<4{Some(n+1)}else{None}).is_err(){reply(request,429,serde_json::json!({"error":"Too many uploads"}));return;}
    let _slot=Slot;
    let name=match request.headers().iter().find(|h|h.field.equiv("X-LLM-Wiki-Filename")).ok_or("Filename missing".to_string()).and_then(|h|filename(h.value.as_str())){
        Ok(v)=>v,Err(e)=>{reply(request,400,serde_json::json!({"error":e}));return;}
    };
    let length=request.body_length();
    if length.map(|n|n as u64>LIMIT).unwrap_or(false){reply(request,413,serde_json::json!({"error":"Single-file upload limit is 100 MiB"}));return;}
    let mut stage:Option<PathBuf>=None;
    let result=(||->Result<(PathBuf,u64),String>{
        let base=root.join(".uploads");
        let mut builder=std::fs::DirBuilder::new();
        #[cfg(unix)]{use std::os::unix::fs::DirBuilderExt;builder.mode(0o700);}
        match builder.create(&base){Ok(_)=>{},Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>{},Err(e)=>return Err(e.to_string())}
        if std::fs::symlink_metadata(&base).map_err(|e|e.to_string())?.file_type().is_symlink()||!base.canonicalize().map_err(|e|e.to_string())?.starts_with(root){return Err("Upload directory rejected".into());}
        let directory=base.join(Uuid::new_v4().to_string());builder.create(&directory).map_err(|e|e.to_string())?;stage=Some(directory.clone());
        let path=directory.join(&name);let mut options=std::fs::OpenOptions::new();options.write(true).create_new(true);
        #[cfg(unix)]{use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
        let mut file=options.open(&path).map_err(|e|e.to_string())?;
        let size=std::io::copy(&mut request.as_reader().take(LIMIT+1),&mut file).map_err(|e|e.to_string())?;
        if size>LIMIT{return Err("Single-file upload limit is 100 MiB".into());}
        if length.map(|n|n as u64!=size).unwrap_or(false){return Err("Incomplete upload".into());}
        file.sync_all().map_err(|e|e.to_string())?;
        Ok((path,size))
    })();
    match result{
        Ok((path,size))=>reply(request,201,serde_json::json!({"path":path,"size":size})),
        Err(error)=>{if let Some(directory)=stage{let _=std::fs::remove_dir_all(directory);}let status=if error.contains("100 MiB"){413}else{400};reply(request,status,serde_json::json!({"error":error}));}
    }
}
