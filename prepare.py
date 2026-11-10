#!/usr/bin/env python3
"""Assemble an external Web host around a pinned, untouched Git source archive."""
import argparse,io,json,re,shutil,subprocess,tarfile
from pathlib import Path

PIN='e08cfce65aad8b76b13546a081c1a8d4422b70c3'
HERE=Path(__file__).resolve().parent
UPSTREAM_URL='https://github.com/nashsu/llm_wiki.git'

def upstream_source(source=None,cache=None):
    if source:return Path(source).resolve()
    cache=Path(cache or HERE/'.cache/upstream.git').resolve()
    if not cache.exists():
        cache.parent.mkdir(parents=True,exist_ok=True)
        subprocess.run(['git','init','--bare',str(cache)],check=True,stdout=subprocess.DEVNULL)
    present=subprocess.run(['git','-C',str(cache),'cat-file','-e',PIN+'^{commit}'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode==0
    if not present:
        subprocess.run(['git','-C',str(cache),'fetch','--depth=1',UPSTREAM_URL,PIN],check=True,timeout=120)
    return cache

def prepare(source,output,node_modules=None,build_ca=None):
    source=Path(source).resolve();output=Path(output).resolve()
    ca=Path(build_ca).read_bytes() if build_ca else b''
    if ca and (b'-----BEGIN CERTIFICATE-----' not in ca or b'PRIVATE KEY' in ca):
        raise ValueError('Build CA must contain public PEM certificates only')
    revision=subprocess.check_output(['git','-C',str(source),'rev-parse',PIN+'^{commit}'],text=True).strip()
    assert revision==PIN
    if output.exists():raise ValueError('Output already exists; refusing to overwrite it')
    entries=[]
    for entry in subprocess.check_output(['git','-C',str(source),'ls-tree','-r','-z',PIN]).split(b'\0'):
        if not entry:continue
        header,name=entry.split(b'\t',1);mode,kind,digest=header.decode().split()
        if kind!='blob':raise ValueError('Submodule sources require explicit handling')
        entries.append({'path':name.decode(),'mode':mode,'gitBlob':digest})
    archive=subprocess.check_output(['git','-C',str(source),'archive',PIN])
    output.mkdir(parents=True);upstream=output/'upstream';upstream.mkdir()
    (output/'upstream-source.tar').write_bytes(archive)
    (output/'build-ca.crt').write_bytes(ca)
    with tarfile.open(fileobj=io.BytesIO(archive)) as bundle:bundle.extractall(upstream,filter='data')
    for name in ['backend','platform']:shutil.copytree(HERE/'templates'/name,output/name)
    for name in ['vite.config.mjs','tsconfig.json']:shutil.copy2(HERE/'templates'/name,output/name)
    for name in ['verify_source.py','run_probe.py']:shutil.copy2(HERE/name,output/name)
    package=json.loads((upstream/'package.json').read_text())
    package['scripts']={'build':'vite build --config vite.config.mjs','typecheck':'tsc --project tsconfig.json'}
    (output/'package.json').write_text(json.dumps(package,indent=2)+'\n')
    shutil.copy2(upstream/'package-lock.json',output/'package-lock.json')
    manifest=(upstream/'src-tauri/Cargo.toml').read_text()
    manifest,count=re.subn(r'\[lib\][\s\S]*?(?=\[build-dependencies\])','[[bin]]\nname = "external-host-smoke"\npath = "main.rs"\n\n[[bin]]\nname = "external-watcher-smoke"\npath = "watcher_smoke.rs"\n\n',manifest)
    if count!=1:raise ValueError('Pinned Cargo layout no longer matches adapter')
    (output/'backend/Cargo.toml').write_text(manifest)
    shutil.copy2(upstream/'src-tauri/Cargo.lock',output/'backend/Cargo.lock')
    config=json.loads((upstream/'src-tauri/tauri.conf.json').read_text())
    config['bundle']['resources']=[];config['bundle']['icon']=[];config['build']['frontendDist']='../dist'
    (output/'backend/tauri.conf.json').write_text(json.dumps(config,indent=2)+'\n')
    (output/'upstream-manifest.json').write_text(json.dumps({'revision':PIN,'files':entries},indent=2)+'\n')
    shutil.copy2(HERE/'Dockerfile',output/'Dockerfile')
    allowed={'build-ca.crt','Dockerfile','.dockerignore','upstream-source.tar','upstream-manifest.json',
        'package.json','package-lock.json','vite.config.mjs','tsconfig.json','verify_source.py','run_probe.py',
        'backend/Cargo.toml','backend/Cargo.lock','backend/tauri.conf.json'}
    allowed.update(str(path.relative_to(HERE/'templates')) for path in (HERE/'templates').rglob('*') if path.is_file())
    directories={str(parent)+'/' for path in allowed for parent in Path(path).parents if str(parent)!='.'}
    patterns=['**']
    for directory in sorted(directories,key=lambda value:(value.count('/'),value)):
        patterns.extend(['!'+directory,directory+'**'])
    patterns.extend('!'+name for name in sorted(allowed))
    (output/'.dockerignore').write_text('\n'.join(patterns)+'\n')
    if node_modules:
        cache=Path(node_modules).resolve()
        if not (cache/'.bin/vite').exists():raise ValueError('Node dependency cache lacks Vite')
        (output/'node_modules').symlink_to(cache,target_is_directory=True)
    print(f'Prepared {len(entries)} unchanged upstream files at {output}')

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source',help='Offline local Git repository containing the pinned revision; otherwise fetch upstream into cache')
    parser.add_argument('--upstream-cache',help='Upstream bare Git cache (default: .cache/upstream.git)')
    parser.add_argument('--output',required=True,help='New assembly directory; never overwritten')
    parser.add_argument('--node-modules',help='Explicit development dependency cache; omit for npm ci')
    parser.add_argument('--build-ca',help='Optional public PEM CA bundle for build stages only; never a private key')
    args=parser.parse_args();prepare(upstream_source(args.source,args.upstream_cache),args.output,args.node_modules,args.build_ca)
