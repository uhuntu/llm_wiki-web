//! Authenticated model transport: exact saved endpoints, original provider auth.
use super::*;
use reqwest::header::{HeaderMap,HeaderName,HeaderValue};

struct Active<'a>(&'a std::sync::atomic::AtomicUsize);
impl Drop for Active<'_>{fn drop(&mut self){self.0.fetch_sub(1,Ordering::SeqCst);}}
fn blocked(name:&str)->bool{
    matches!(name,"host"|"connection"|"content-length"|"transfer-encoding"|"proxy-authorization"|"proxy-connection"|"upgrade"|"te"|"trailer"|"cookie"|"set-cookie"|"authorization"|"api-key"|"x-api-key"|"x-goog-api-key"|"origin")
}
pub fn relay(mut request:tiny_http::Request,state:&Probe){
    if state.relay_active.fetch_update(Ordering::SeqCst,Ordering::SeqCst,|n|if n<8{Some(n+1)}else{None}).is_err(){
        reply(request,429,serde_json::json!({"error":"Too many active model requests"}));return;
    }
    let _active=Active(&state.relay_active);
    let config=match state.active_model(){Ok(value)=>value,Err(error)=>{reply(request,503,serde_json::json!({"error":error}));return;}};
    let routes=match agent::provider::external_http_requests(&config){Ok(value)=>value,Err(error)=>{reply(request,400,serde_json::json!({"error":error}));return;}};
    let target=request.headers().iter().find(|h|h.field.equiv("X-LLM-Wiki-URL")).map(|h|h.value.as_str()).unwrap_or("");
    let parsed=match reqwest::Url::parse(target){Ok(url)=>url,Err(_)=>{reply(request,403,serde_json::json!({"error":"Invalid model URL"}));return;}};
    let authoritative=match routes.into_iter().find(|(url,_)|url==&parsed){Some((_,headers))=>headers,None=>{reply(request,403,serde_json::json!({"error":"Request does not match the saved model endpoint"}));return;}};
    let method=request.headers().iter().find(|h|h.field.equiv("X-LLM-Wiki-Method")).map(|h|h.value.as_str()).unwrap_or("POST");
    if method!="POST"{reply(request,405,serde_json::json!({"error":"Only model generation POST requests are supported"}));return;}
    let mut headers=HeaderMap::new();
    if let Some(raw)=request.headers().iter().find(|h|h.field.equiv("X-LLM-Wiki-Headers")){
        let values:std::collections::BTreeMap<String,String>=match serde_json::from_str(raw.value.as_str()){
            Ok(value)=>value,Err(_)=>{reply(request,400,serde_json::json!({"error":"Invalid model request headers"}));return;}
        };
        for (name,value) in values{
            let name=match HeaderName::from_bytes(name.as_bytes()){Ok(name)=>name,Err(_)=>{reply(request,400,serde_json::json!({"error":"Invalid model request header name"}));return;}};
            if blocked(name.as_str()){continue;}
            let value=match HeaderValue::from_str(&value){Ok(value)=>value,Err(_)=>{reply(request,400,serde_json::json!({"error":"Invalid model request header value"}));return;}};
            headers.insert(name,value);
        }
    }
    // Browser-supplied credentials never replace the saved provider settings.
    // These headers come from the original backend provider's own builders.
    headers.extend(authoritative);
    let mut body=Vec::new();
    if request.as_reader().take(8_388_609).read_to_end(&mut body).is_err()||body.len()>8_388_608{reply(request,413,serde_json::json!({"error":"Model request exceeds 8 MiB"}));return;}
    if !serde_json::from_slice::<serde_json::Value>(&body).is_ok_and(|value|value.is_object()){
        reply(request,400,serde_json::json!({"error":"Model request must be a JSON object"}));return;
    }
    let result=tauri::async_runtime::block_on(async{
        let builder=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(std::time::Duration::from_secs(180));
        let builder=if state.is_fixture(){builder.no_proxy()}else{crate::proxy::configure_http_client(builder)};
        builder.build().map_err(|_|"Cannot initialize model HTTP client".to_string())?
            .post(parsed).headers(headers).body(body).send().await.map_err(|error|error.without_url().to_string())
    });
    let response=match result{Ok(response)=>response,Err(error)=>{reply(request,502,serde_json::json!({"error":error}));return;}};
    let status=response.status().as_u16();
    let kind=response.headers().get("Content-Type").and_then(|v|v.to_str().ok()).unwrap_or("application/octet-stream").to_string();
    let retry=response.headers().get("Retry-After").and_then(|v|v.to_str().ok()).map(|v|format!("Retry-After: {v}\r\n")).unwrap_or_default();
    let mut writer=request.into_writer();
    if write!(writer,"HTTP/1.1 {status} Model\r\nContent-Type: {kind}\r\n{retry}Cache-Control: no-store\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").and_then(|_|writer.flush()).is_err(){return;}
    tauri::async_runtime::block_on(async{
        use futures::StreamExt;
        let mut chunks=response.bytes_stream();
        while let Some(chunk)=chunks.next().await{
            match chunk{
                Ok(bytes)=>{
                    if bytes.is_empty(){continue;}
                    if write!(writer,"{:X}\r\n",bytes.len()).and_then(|_|writer.write_all(&bytes)).and_then(|_|writer.write_all(b"\r\n")).and_then(|_|writer.flush()).is_err(){return;}
                }
                // Leave the chunked response incomplete so fetch rejects a
                // truncated stream, instead of fabricating a model response.
                Err(_)=>return,
            }
        }
        let _=writer.write_all(b"0\r\n\r\n").and_then(|_|writer.flush());
    });
}
