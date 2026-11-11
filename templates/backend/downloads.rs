//! Authenticated, workspace-confined file delivery for the original UI's asset URLs.
use super::*;
use std::path::{Component,Path};

fn encoded_name(name:&str)->String{
    let mut out=String::new();
    for byte in name.bytes(){
        if byte.is_ascii_alphanumeric()||matches!(byte,b'-'|b'.'|b'_'|b'~'){out.push(byte as char)}
        else{out.push_str(&format!("%{byte:02X}"));}
    }
    out
}
fn content_type(path:&Path)->&'static str{
    match path.extension().and_then(|e|e.to_str()).unwrap_or("").to_ascii_lowercase().as_str(){
        "html"|"htm"=>"text/html; charset=utf-8","txt"|"md"|"csv"|"log"|"json"|"xml"=>"text/plain; charset=utf-8",
        "png"=>"image/png","jpg"|"jpeg"=>"image/jpeg","gif"=>"image/gif","webp"=>"image/webp","svg"=>"image/svg+xml",
        "pdf"=>"application/pdf","mp3"=>"audio/mpeg","wav"=>"audio/wav","mp4"=>"video/mp4","webm"=>"video/webm",
        "zip"=>"application/zip","jsonl"=>"application/x-ndjson",_=>"application/octet-stream"
    }
}
pub fn serve(request:tiny_http::Request,root:&Path){
    let response=(||->Result<tiny_http::Response<std::fs::File>,(u16,String)>{
        let url=reqwest::Url::parse(&format!("http://localhost{}",request.url())).map_err(|_|(400,"Invalid asset URL".into()))?;
        let mut values=url.query_pairs().filter(|(key,_)|key=="path").map(|(_,value)|value.into_owned());
        let raw=values.next().ok_or((400,"File path missing".into()))?;
        if values.next().is_some()||url.query_pairs().any(|(key,_)|key!="path"){return Err((400,"Unexpected asset query".into()));}
        let candidate=Path::new(&raw);
        if !candidate.is_absolute()||candidate.components().any(|part|matches!(part,Component::ParentDir))||!candidate.starts_with(root){return Err((403,"File path rejected".into()));}
        let path=candidate.canonicalize().map_err(|_|(404,"File not found".into()))?;
        if !path.starts_with(root)||!path.is_file(){return Err((403,"File path rejected".into()));}
        let file=std::fs::File::open(&path).map_err(|_|(404,"File not found".into()))?;
        let filename=path.file_name().and_then(|n|n.to_str()).unwrap_or("download");
        let mut response=tiny_http::Response::from_file(file).with_header(tiny_http::Header::from_bytes("Content-Type",content_type(&path)).unwrap());
        response.add_header(tiny_http::Header::from_bytes("Content-Disposition",format!("inline; filename*=UTF-8''{}",encoded_name(filename))).unwrap());
        response.add_header(tiny_http::Header::from_bytes("Cache-Control","private, no-store").unwrap());
        response.add_header(tiny_http::Header::from_bytes("X-Content-Type-Options","nosniff").unwrap());
        if matches!(path.extension().and_then(|e|e.to_str()).unwrap_or("").to_ascii_lowercase().as_str(),"html"|"htm"|"svg"|"xml"){
            response.add_header(tiny_http::Header::from_bytes("Content-Security-Policy","default-src 'none'; sandbox").unwrap());
        }
        Ok(response)
    })();
    match response{Ok(response)=>{let _=request.respond(response);},Err((status,error))=>reply(request,status,error_value(error))}
}
fn error_value(message:String)->serde_json::Value{serde_json::json!({"error":message})}
