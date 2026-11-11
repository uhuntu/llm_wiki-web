import subprocess,select,re,http.cookiejar,urllib.request,urllib.error,json
from pathlib import Path
root=Path('/data');root.mkdir(exist_ok=True)
p=subprocess.Popen(['python3','/opt/llmwiki/run_probe.py','--binary','/opt/llmwiki/bin/external-host','--workspace','/data','--mode','live','--port','18962'],stdout=subprocess.PIPE,text=True)
try:
 assert select.select([p.stdout],[],[],20)[0]
 ticket=re.search(r'http://localhost:\d+/login\?ticket=[0-9a-f-]+',p.stdout.readline()).group()
 jar=http.cookiejar.CookieJar();client=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar));client.open(ticket).close()
 f=root/'中文 + %.txt';raw='Synthetic download 青绿色'.encode();f.write_bytes(raw)
 url='http://localhost:18962/_llmwiki/asset?path='+urllib.parse.quote(str(f),safe='')
 with client.open(url) as response:
  assert response.read()==raw
  assert response.headers['X-Content-Type-Options']=='nosniff'
  assert "filename*=UTF-8''" in response.headers['Content-Disposition']
 def rejected(url,opener,code):
  try:opener.open(url);raise AssertionError('Request accepted')
  except urllib.error.HTTPError as e:assert e.code==code,e.code
 rejected(url,urllib.request.build_opener(),401)
 for path in ['/etc/hostname','/data/../etc/hostname']:
  rejected('http://localhost:18962/_llmwiki/asset?path='+urllib.parse.quote(path,safe=''),client,403)
 (root/'escape').symlink_to('/etc/hostname')
 rejected('http://localhost:18962/_llmwiki/asset?path=/data/escape',client,403)
 html=root/'active.html';html.write_text('<script>alert(1)</script>')
 with client.open('http://localhost:18962/_llmwiki/asset?path=/data/active.html') as response:assert 'sandbox' in response.headers['Content-Security-Policy']
 print('PASS: exact UTF-8 file bytes, filename/header delivery, login enforcement, traversal and symlink rejection, HTML sandbox')
finally:p.terminate();p.wait(timeout=10)
