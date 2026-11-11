import subprocess,select,re,json,http.cookiejar,urllib.request
from pathlib import Path
p=None
root=Path('/data')
def start():
 global p,client
 p=subprocess.Popen(['python3','/opt/llmwiki/run_probe.py','--binary','/opt/llmwiki/bin/external-host','--workspace','/data','--mode','live','--port','18982'],stdout=subprocess.PIPE,text=True)
 assert select.select([p.stdout],[],[],20)[0],'Startup timed out'
 line=p.stdout.readline();ticket=re.search(r'http://localhost:\d+/login\?ticket=[0-9a-f-]+',line);assert ticket,line
 client=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()));client.open(ticket.group()).close()
def stop():
 global p
 p.terminate();p.wait(timeout=10);p=None
def invoke(command,args):
 req=urllib.request.Request('http://localhost:18982/_llmwiki/invoke',data=json.dumps({'command':command,'args':args}).encode(),headers={'Content-Type':'application/json'})
 with client.open(req) as response:
  value=json.load(response);assert value['ok'],value;return value['value']
def store(action,key,value=None):return invoke('_web_store',{'store':'app-state.json','action':action,'key':key,'value':value})
try:
 start();project=store('get','lastProject');registry=store('get','projectRegistry')
 config={'provider':'custom','model':'lifecycle-sentinel','apiKey':'synthetic-not-real','customEndpoint':'http://127.0.0.1:1/v1'}
 store('set','llmConfig',config);file=project['path']+'/wiki/lifecycle.md';invoke('write_file',{'path':file,'contents':'Synthetic lifecycle content'})
 store('set','lastProject',None);stop();before={p.name for p in root.iterdir() if p.is_dir()}
 start();assert store('get','lastProject') is None;assert store('get','projectRegistry')==registry;assert store('get','llmConfig')==config
 assert {p.name for p in root.iterdir() if p.is_dir()}==before
 assert invoke('read_file',{'path':file})=='Synthetic lifecycle content'
 opened=invoke('open_project',{'path':project['path']});assert opened['path']==project['path'];store('set','lastProject',project);stop()
 start();assert store('get','lastProject')==project;assert store('get','llmConfig')==config
 print('PASS: close/restart without an active project; no new project created; registry, credentials and files preserved; reopen and restart restored')
finally:
 if p is not None:stop()
