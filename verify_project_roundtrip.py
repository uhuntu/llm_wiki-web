import subprocess,select,re,shutil,zipfile,json,http.cookiejar,urllib.request,urllib.error
from pathlib import Path
shutil.copyfile('/input.zip','/data/browser-export.zip')
p=subprocess.Popen(['python3','/opt/llmwiki/run_probe.py','--binary','/opt/llmwiki/bin/external-host','--workspace','/data','--mode','live','--port','18972'],stdout=subprocess.PIPE,text=True)
try:
 assert select.select([p.stdout],[],[],20)[0]
 ticket=re.search(r'http://localhost:\d+/login\?ticket=[0-9a-f-]+',p.stdout.readline()).group()
 client=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()));client.open(ticket).close()
 def invoke(command,args):
  req=urllib.request.Request('http://localhost:18972/_llmwiki/invoke',data=json.dumps({'command':command,'args':args}).encode(),headers={'Content-Type':'application/json'})
  with client.open(req) as res:
   data=json.load(res);assert data['ok'],data;return data['value']
 path=invoke('import_project_archive',{'archivePath':'/data/browser-export.zip','destination':'/data/Restored Wiki'})
 with zipfile.ZipFile('/input.zip') as z:
  count=0
  for item in z.infolist():
   if not item.is_dir():assert (Path(path)/item.filename).read_bytes()==z.read(item);count+=1
 print('PASS: browser-export archive imported using original Rust command;',count,'files match byte for byte')
 try:invoke('import_project_archive',{'archivePath':'/data/browser-export.zip','destination':'/data/Restored Wiki'});raise AssertionError('Nonempty target overwritten')
 except urllib.error.HTTPError as e:assert e.code==400
 project=invoke('open_project',{'path':path});assert project['path']==path
 print('PASS: restored project opened; nonempty destination rejected')
finally:p.terminate();p.wait(timeout=10)
