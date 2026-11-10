// Included after the untouched provider.rs, in its own module scope.
// Reuse the author's normalization and authentication rules for browser HTTP
// traffic as well as Agent requests. No independent copy of those rules.
pub(crate) fn external_http_requests(config:&LlmConfig)->Result<Vec<(reqwest::Url,HeaderMap)>,String>{
    if !config.is_usable_for_backend_http(){
        return Err("Configure a supported HTTP model in Settings before sending a request".into());
    }
    let mut requests=Vec::new();
    match config.provider.as_str(){
        "openai"=>{let url="https://api.openai.com/v1/chat/completions";requests.push((url.to_string(),openai_headers(config,url)?));}
        "azure"=>{let url=build_azure_url(config)?;let headers=openai_headers(config,&url)?;requests.push((url,headers));}
        "ollama"=>{let url=build_ollama_url(&config.ollama_url);let headers=openai_headers(config,&url)?;requests.push((url,headers));}
        "custom" if config.api_mode.as_deref()==Some("anthropic_messages")=>{
            let url=build_anthropic_url(&config.custom_endpoint);let headers=anthropic_headers(config,&url)?;requests.push((url,headers));
        }
        "custom"=>{let url=build_custom_openai_url(config);let headers=openai_headers(config,&url)?;requests.push((url,headers));}
        "anthropic"|"minimax"=>{
            let base=if config.provider=="anthropic"{"https://api.anthropic.com"}else if config.custom_endpoint.trim().is_empty(){"https://api.minimax.io/anthropic"}else{&config.custom_endpoint};
            let url=build_anthropic_url(base);let headers=anthropic_headers(config,&url)?;requests.push((url,headers));
        }
        "google"=>{
            let model=url_encode_path_segment(&config.model);
            let base=format!("https://generativelanguage.googleapis.com/v1beta/models/{model}");
            requests.push((format!("{base}:generateContent"),google_headers(config)?));
            requests.push((format!("{base}:streamGenerateContent?alt=sse"),google_headers(config)?));
        }
        _=>return Err("This provider requires a transport not available in the Web host".into()),
    }
    requests.into_iter().map(|(url,headers)|{
        let url=reqwest::Url::parse(&url).map_err(|_|"Invalid configured model URL")?;
        if !matches!(url.scheme(),"http"|"https")||url.host_str().is_none()||!url.username().is_empty()||url.password().is_some()||url.fragment().is_some(){
            return Err("Model URL must use HTTP(S), with no embedded credentials or fragment".into());
        }
        for name in headers.keys(){
            if matches!(name.as_str(),"host"|"connection"|"content-length"|"transfer-encoding"|"proxy-authorization"|"proxy-connection"|"upgrade"|"te"|"trailer"){
                return Err(format!("Unsupported model transport header: {name}"));
            }
        }
        Ok((url,headers))
    }).collect()
}
