#!/usr/bin/env python3
"""Run inside an isolated container with prepared sources at /delivery."""
from pathlib import Path
import http.cookiejar,json,os,re,select,subprocess,sys,urllib.error,urllib.request
root=Path(sys.argv[1]);binary=Path(sys.argv[2]);port=18912
workspace=Path(sys.argv[3]) if len(sys.argv)>3 else root/'fixtures'
workspace.mkdir(exist_ok=True,mode=0o700)
env=os.environ.copy();env['EXTERNAL_HOST_FIXTURE_ROOT']=str(workspace)
subprocess.run([str(binary)],env=env,check=True,timeout=30)

def start():
    process=subprocess.Popen([sys.executable,str(root/'run_probe.py'),'--binary',str(binary),'--port',str(port),'--workspace',str(workspace)],stdout=subprocess.PIPE,text=True)
    try:
        if not select.select([process.stdout],[],[],15)[0]:raise RuntimeError('Probe startup timed out')
        line=process.stdout.readline();url=re.search(r'http://localhost:\d+/login\?ticket=[0-9a-f-]+',line)
        if not url:raise RuntimeError('Probe failed to print login URL')
        opener=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
        with opener.open(url.group(0),timeout=10) as response:html=response.read().decode()
        return process,opener,html
    except BaseException:
        process.terminate();process.wait(timeout=5);raise

def invoke(opener,command,args):
    request=urllib.request.Request(f'http://localhost:{port}/_llmwiki/invoke',data=json.dumps({'command':command,'args':args}).encode(),headers={'Content-Type':'application/json'})
    with opener.open(request,timeout=10) as response:result=json.load(response)
    assert result['ok'],result
    return result['value']

def stop(process):
    process.terminate();process.wait(timeout=5)

process=None
try:
    process,opener,html=start()
    assert '<div id="root"></div>' in html
    asset=re.search(r'src="(/assets/[^" ]+\.js)"',html).group(1)
    with opener.open(f'http://localhost:{port}'+asset,timeout=5) as response:assert response.status==200 and 'javascript' in response.headers['Content-Type']
    info=invoke(opener,'_web_info',{});assert info['root']==str(workspace)
    store=lambda key:invoke(opener,'_web_store',{'store':'app-state.json','action':'get','key':key})
    project=store('lastProject');file=project['path']+'/wiki/relocation-proof.md'
    invoke(opener,'write_file',{'path':file,'contents':'Synthetic relocated host fixture'})
    invoke(opener,'_web_store',{'store':'app-state.json','action':'set','key':'relocationFixture','value':True})
    try:invoke(opener,'read_file',{'path':'/etc/hostname'});raise AssertionError('Filesystem boundary bypassed')
    except urllib.error.HTTPError as error:assert error.code==400
    stop(process);process=None
    process,opener,_=start()
    assert invoke(opener,'_web_store',{'store':'app-state.json','action':'get','key':'relocationFixture'}) is True
    restored=invoke(opener,'_web_store',{'store':'app-state.json','action':'get','key':'lastProject'})
    assert restored['path']==project['path'] and restored['id']==project['id']
    assert invoke(opener,'read_file',{'path':file})=='Synthetic relocated host fixture'
    assert (workspace/'external-ui-state.json').stat().st_mode&0o777==0o600
    subprocess.run([sys.executable,str(root/'verify_source.py')],check=True)
    print('PASS: relocated runner, original UI assets, file RPC, source boundary, settings/project restart and child shutdown without old mount paths')
finally:
    if process is not None:stop(process)
