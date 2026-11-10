#!/usr/bin/env python3
"""Run the external host with explicitly separate fixture/live model modes."""
import argparse,json,os,signal,subprocess
from pathlib import Path
from verify_source import verify

def claim_workspace_mode(workspace,mode):
    """Prevent the compatibility fixture default from resetting live settings."""
    marker=workspace/'.external-host-model-mode'
    if marker.is_symlink():raise ValueError('Workspace mode marker must not be a symlink')
    if marker.exists():
        if marker.read_text().strip()!=mode:
            raise ValueError('Use separate workspaces for live and fixture modes')
        return
    settings=workspace/'external-ui-state.json'
    if settings.exists():
        value=json.loads(settings.read_text())
        if not isinstance(value,dict):raise ValueError('Saved settings must be an object')
        config=value.get('llmConfig',{})
        is_fixture=isinstance(config,dict) and config.get('provider')=='custom' and config.get('model')=='fixture' and not config.get('apiKey')
        if (mode=='fixture')!=is_fixture:
            raise ValueError('Existing model configuration belongs to the other mode; use a separate workspace')
    try:fd=os.open(marker,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
    except FileExistsError:
        if marker.is_symlink() or marker.read_text().strip()!=mode:
            raise ValueError('Workspace mode changed concurrently')
        return
    with os.fdopen(fd,'w') as stream:
        stream.write(mode+'\n');stream.flush();os.fsync(stream.fileno())

if __name__=='__main__':
    root=Path(__file__).resolve().parent
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,default=root/'backend/target/release/external-host-smoke')
    parser.add_argument('--workspace',type=Path,default=root/'fixtures')
    parser.add_argument('--port',type=int,default=18902)
    parser.add_argument('--mode',choices=['fixture','live'],default='fixture')
    args=parser.parse_args()
    if not 1024<=args.port<=65535:parser.error('Use an unprivileged TCP port')
    verify(root)
    if not args.binary.is_file() or not (root/'dist/index.html').is_file():parser.error('Build the frontend and backend first')
    workspace=args.workspace.resolve();workspace.mkdir(parents=True,exist_ok=True,mode=0o700)
    try:claim_workspace_mode(workspace,args.mode)
    except (OSError,ValueError) as error:parser.error(str(error))
    env=os.environ.copy();env.update(EXTERNAL_HOST_HTTP_PROBE='1',EXTERNAL_HOST_PROBE_PORT=str(args.port),
        EXTERNAL_HOST_FIXTURE_ROOT=str(workspace),EXTERNAL_HOST_STATIC_ROOT=str(root/'dist'),
        EXTERNAL_HOST_MODEL_MODE=args.mode,
        PDFIUM_DYNAMIC_LIB_PATH=str(root/'upstream/src-tauri/pdfium/libpdfium.so'))
    env['NO_PROXY']=','.join(filter(None,[env.get('NO_PROXY',''),env.get('no_proxy',''),'localhost,127.0.0.1,::1']))
    env['no_proxy']=env['NO_PROXY']
    child=subprocess.Popen([str(args.binary.resolve())],env=env,start_new_session=True)
    def stop(signum,_frame):
        if child.poll() is None:
            try:os.killpg(child.pid,signum)
            except ProcessLookupError:pass
    signal.signal(signal.SIGTERM,stop);signal.signal(signal.SIGINT,stop)
    try:code=child.wait()
    finally:
        if child.poll() is None:
            stop(signal.SIGTERM,None)
            try:child.wait(timeout=5)
            except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);child.wait()
    raise SystemExit(code if code>=0 else 128-code)
