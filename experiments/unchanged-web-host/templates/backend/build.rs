use std::{env,fs,path::PathBuf};

fn main() {
    let host=PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let source=host.join("../upstream/src-tauri/src").canonicalize().expect("prepare upstream source first");
    let mut graph=String::new();
    graph.push_str(&format!("#[path={:?}] mod types;\n",source.join("types/mod.rs")));
    // Compose the original Agent modules externally, exactly as for commands.
    // The provider extension can reuse private URL/header helpers without
    // editing, copying or replacing any original provider implementation.
    let agent_modules=source.join("agent/mod.rs");
    println!("cargo:rerun-if-changed={}",agent_modules.display());
    graph.push_str("mod agent {\n");
    for line in fs::read_to_string(&agent_modules).unwrap().lines(){
        let line=line.trim();
        if line.is_empty()||line.starts_with("//"){continue;}
        if let Some(name)=line.strip_prefix("pub mod ").and_then(|v|v.strip_suffix(';')){
            assert!(name.chars().all(|c|c.is_ascii_alphanumeric()||c=='_'));
            if name=="provider"{
                graph.push_str(&format!("pub mod provider {{ include!({:?}); include!({:?}); }}\n",source.join("agent/provider.rs"),host.join("provider_extension.rs")));
            }else{graph.push_str(&format!("#[path={:?}] pub mod {};\n",source.join("agent").join(format!("{name}.rs")),name));}
        }else if line.starts_with("pub use ")&&line.ends_with(';'){graph.push_str(line);graph.push('\n');}
        else{panic!("unsupported upstream Agent declaration: {line}");}
    }
    graph.push_str("}\n");
    for name in ["panic_guard","proxy"]{graph.push_str(&format!("#[path={:?}] mod {};\n",source.join(format!("{name}.rs")),name));}
    let modules=source.join("commands/mod.rs");
    println!("cargo:rerun-if-changed={}",modules.display());
    graph.push_str("mod commands {\n");
    for line in fs::read_to_string(&modules).unwrap().lines(){
        let line=line.trim();
        let (visibility,name)=if let Some(value)=line.strip_prefix("pub mod "){("pub ",value)}else if let Some(value)=line.strip_prefix("mod "){("",value)}else if line.is_empty()||line.starts_with("//"){continue;}else{panic!("unsupported upstream module declaration: {line}")};
        let name=name.strip_suffix(';').expect("module declaration must end with semicolon");
        assert!(name.chars().all(|ch|ch.is_ascii_alphanumeric()||ch=='_'),"invalid module name");
        if name=="file_sync"{
            graph.push_str(&format!("pub mod file_sync {{ include!({:?}); include!({:?}); }}\n",source.join("commands/file_sync.rs"),host.join("watcher_extension.rs")));
        }else{graph.push_str(&format!("#[path={:?}] {}mod {};\n",source.join("commands").join(format!("{name}.rs")),visibility,name));}
    }
    graph.push_str("}\n");
    fs::write(PathBuf::from(env::var("OUT_DIR").unwrap()).join("host_module_graph.rs"),graph).unwrap();
    let windows=tauri_build::WindowsAttributes::new().app_manifest(include_str!("../upstream/src-tauri/windows-app-manifest.xml"));
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows)).expect("Tauri dependency build metadata");
}
