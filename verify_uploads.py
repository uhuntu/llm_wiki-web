#!/usr/bin/env python3
"""Check authenticated uploads against the packaged host, using synthetic bytes only."""
import argparse,hashlib,http.client,http.cookiejar,json,os,re,select,socket,subprocess,urllib.error,urllib.parse,urllib.request
from pathlib import Path

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('root',type=Path);p.add_argument('binary',type=Path);p.add_argument('--workspace',type=Path,required=True);p.add_argument('--port',type=int,default=18952);a=p.parse_args()
    a.workspace.mkdir(parents=True,exist_ok=True,mode=0o700)
    process=subprocess.Popen(['python3',str(a.root/'run_probe.py'),'--binary',str(a.binary),'--workspace',str(a.workspace),'--mode','live','--port',str(a.port)],stdout=subprocess.PIPE,text=True)
    try:
        assert select.select([process.stdout],[],[],20)[0],'Startup timeout'
        ticket=re.search(r'http://localhost:\d+/login\?ticket=[0-9a-f-]+',process.stdout.readline());assert ticket
        jar=http.cookiejar.CookieJar();opener=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
        with opener.open(ticket.group(0),timeout=10):pass
        base=f'http://localhost:{a.port}'
        def upload(name,data=b'synthetic',headers=None,authenticated=True):
            h={'Content-Type':'application/octet-stream','X-LLM-Wiki-Filename':name};h.update(headers or {})
            req=urllib.request.Request(base+'/_llmwiki/upload',data=data,headers=h)
            try:
                with (opener if authenticated else urllib.request.build_opener()).open(req,timeout=10) as response:return response.status,json.load(response)
            except urllib.error.HTTPError as error:return error.code,json.load(error)
        assert upload('a.txt',authenticated=False)[0]==401
        assert upload('a.txt',headers={'Origin':'http://foreign.invalid'})[0]==403
        for name in ['..','%2E%2E','..%2Foutside.txt','..%5Coutside.txt','%00bad','%0Abad','%ZZ','%FF','x'*241]:assert upload(name)[0]==400,name
        raw='Synthetic browser upload — 青绿色 + 100%'.encode();name='中文 + 100%.txt'
        status,first=upload(urllib.parse.quote(name,safe=''),raw);assert status==201,first
        status,second=upload(urllib.parse.quote(name,safe=''),b'different');assert status==201 and first['path']!=second['path']
        file=Path(first['path']);assert file.name==name and file.is_relative_to(a.workspace/'.uploads')
        assert file.read_bytes()==raw and first['size']==len(raw)
        assert Path(second['path']).read_bytes()==b'different' and file.stat().st_mode&0o777==0o600
        assert file.parent.stat().st_mode&0o777==0o700
        status,empty=upload('empty.txt',b'');assert status==201 and Path(empty['path']).stat().st_size==0
        connection=http.client.HTTPConnection('localhost',a.port,timeout=10)
        connection.putrequest('POST','/_llmwiki/upload');connection.putheader('Cookie','; '.join(f'{c.name}={c.value}' for c in jar));connection.putheader('X-LLM-Wiki-Filename','large.bin');connection.putheader('Content-Length',str(100*1024*1024+1));connection.endheaders()
        response=connection.getresponse();assert response.status==413;response.read();connection.close()
        # Stream the exact accepted boundary without allocating a 100 MiB byte string.
        connection=http.client.HTTPConnection('localhost',a.port,timeout=30)
        connection.putrequest('POST','/_llmwiki/upload');connection.putheader('Cookie','; '.join(f'{c.name}={c.value}' for c in jar));connection.putheader('X-LLM-Wiki-Filename','boundary.bin');connection.putheader('Content-Length',str(100*1024*1024));connection.endheaders()
        block=b'X'*(1024*1024)
        for _ in range(100):connection.send(block)
        response=connection.getresponse();boundary=json.load(response);assert response.status==201 and boundary['size']==100*1024*1024
        assert Path(boundary['path']).stat().st_size==100*1024*1024;Path(boundary['path']).unlink();Path(boundary['path']).parent.rmdir();connection.close()
        connection=http.client.HTTPConnection('localhost',a.port,timeout=10)
        connection.putrequest('POST','/_llmwiki/upload');connection.putheader('Cookie','; '.join(f'{c.name}={c.value}' for c in jar));connection.putheader('X-LLM-Wiki-Filename','interrupted.txt');connection.putheader('Content-Length','99');connection.endheaders();connection.send(b'partial');connection.sock.shutdown(socket.SHUT_WR)
        try:
            response=connection.getresponse();assert response.status==400;response.read()
        except http.client.RemoteDisconnected:pass  # A truncated HTTP request may be closed without a response.
        finally:connection.close()
        assert not list((a.workspace/'.uploads').glob('*/interrupted.txt'))
        # Never follow a malicious staging-directory symlink outside the workspace.
        basepath=a.workspace/'.uploads';kept=a.workspace/'.uploads-kept';outside=Path('/tmp')/('upload-outside-'+str(os.getpid()));outside.mkdir(exist_ok=True)
        basepath.rename(kept)
        try:
            basepath.symlink_to(outside,target_is_directory=True)
            assert upload('escape.txt')[0]==400 and not list(outside.iterdir())
        finally:basepath.unlink();kept.rename(basepath);outside.rmdir()
        req=urllib.request.Request(base+'/_llmwiki/invoke',data=json.dumps({'command':'read_file','args':{'path':str(file)}}).encode(),headers={'Content-Type':'application/json'})
        with opener.open(req,timeout=10) as response:assert json.load(response)['value']==raw.decode()
        subprocess.run(['python3',str(a.root/'verify_source.py')],check=True)
        print('PASS: authenticated upload, CSRF rejection, UTF-8 names/bytes, traversal rejection, duplicate isolation, private permissions, empty file, 100 MiB boundary, interrupted-upload cleanup, symlink rejection and original read_file')
    finally:process.terminate();process.wait(timeout=10)
if __name__=='__main__':main()
