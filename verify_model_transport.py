#!/usr/bin/env python3
"""Exercise live-mode HTTP/Agent wiring with a local protocol server, not a real model."""
import argparse,http.cookiejar,json,os,re,select,subprocess,threading,time,urllib.error,urllib.request
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root',type=Path);parser.add_argument('binary',type=Path)
    parser.add_argument('--workspace',type=Path,required=True);parser.add_argument('--port',type=int,default=18942)
    args=parser.parse_args();args.workspace.mkdir(parents=True,exist_ok=True,mode=0o700)
    seen=[];slow=threading.Event()
    class Model(BaseHTTPRequestHandler):
        def log_message(self,*_):pass
        def do_POST(self):
            body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            mode=body.get('testMode','ok')
            seen.append({'path':self.path,'auth':self.headers.get('Authorization'),'key':self.headers.get('x-api-key'),'version':self.headers.get('anthropic-version'),'custom':self.headers.get('X-Test-Route'),'cookie':self.headers.get('Cookie')})
            if mode=='slow':slow.set();time.sleep(1.5)
            if mode in ('unauthorized','limited','redirect'):
                status={'unauthorized':401,'limited':429,'redirect':302}[mode]
                data=json.dumps({'error':{'message':mode}}).encode();self.send_response(status)
                if mode=='limited':self.send_header('Retry-After','7')
                if mode=='redirect':self.send_header('Location','http://127.0.0.1:1/never')
                self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data);return
            agent='transport-agent-proof' in json.dumps(body)
            content=json.dumps({'action':'final','answer':'Live Agent configuration confirmed.'}) if agent else '青绿色'
            if body.get('stream',True):
                data=('data: '+json.dumps({'choices':[{'delta':{'content':content}}]},ensure_ascii=False)+'\n\ndata: [DONE]\n\n').encode()
                kind='text/event-stream'
            else:data=json.dumps({'choices':[{'message':{'content':content}}]},ensure_ascii=False).encode();kind='application/json'
            self.send_response(200);self.send_header('Content-Type',kind)
            self.send_header('Content-Length',str(len(data)+(50 if mode=='truncated' else 0)));self.end_headers()
            for n in range(0,len(data),7):self.wfile.write(data[n:n+7]);self.wfile.flush()
            if mode=='truncated':self.close_connection=True
    model=ThreadingHTTPServer(('127.0.0.1',0),Model);threading.Thread(target=model.serve_forever,daemon=True).start()
    origin=f'http://127.0.0.1:{model.server_port}';base=f'http://localhost:{args.port}'
    process=None;opener=None
    def start():
        nonlocal process,opener
        process=subprocess.Popen(['python3',str(args.root/'run_probe.py'),'--binary',str(args.binary),'--mode','live','--port',str(args.port),'--workspace',str(args.workspace)],stdout=subprocess.PIPE,text=True)
        if not select.select([process.stdout],[],[],20)[0]:raise RuntimeError('Web host startup timed out')
        line=process.stdout.readline();ticket=re.search(r'http://localhost:\d+/login\?ticket=[0-9a-f-]+',line)
        assert ticket,'No login ticket received'
        opener=urllib.request.build_opener(urllib.request.ProxyHandler({}),urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
        with opener.open(ticket.group(),timeout=10) as r:r.read()
    def stop():
        nonlocal process
        process.terminate();process.wait(timeout=5);process=None
    def request(path,data=None,headers=None):
        req=urllib.request.Request(base+path,data=data,headers=headers or {})
        try:
            with opener.open(req,timeout=20) as r:return r.status,r.headers,r.read()
        except urllib.error.HTTPError as e:return e.code,e.headers,e.read()
    def invoke(command,values):
        status,_,body=request('/_llmwiki/invoke',json.dumps({'command':command,'args':values}).encode(),{'Content-Type':'application/json'})
        result=json.loads(body);assert status==200 and result['ok'],(status,result);return result['value']
    def store(action,key,value=None):return invoke('_web_store',{'store':'app-state.json','action':action,'key':key,'value':value})
    def relay(target,mode='ok',stream=True,extra=None):
        headers={'X-LLM-Wiki-URL':target,'X-LLM-Wiki-Method':'POST','X-LLM-Wiki-Headers':json.dumps(extra or {})}
        return request('/_llmwiki/http',json.dumps({'messages':[],'testMode':mode,'stream':stream}).encode(),headers)
    try:
        start();assert invoke('_web_info',{})['modelMode']=='live'
        assert relay(origin+'/v1/chat/completions')[0]==503
        assert request('/probe-trace')[0]==404 and request('/run',b'{}')[0]==404
        cfg={'provider':'custom','model':'transport-test','apiKey':'synthetic-key-not-real','customEndpoint':origin+'/gateway/v1','ollamaUrl':'','maxTokens':128,'streamingEnabled':True,'customHeaders':{'X-Test-Route':'saved'}}
        store('set','llmConfig',cfg);target=origin+'/gateway/v1/chat/completions'
        status,_,data=relay(target,extra={'Authorization':'Bearer browser-key-must-not-win','Cookie':'must-not-forward','Host':'bad.invalid','X-Test-Route':'browser'})
        assert status==200 and '青绿色' in data.decode() and b'[DONE]' in data
        assert seen[-1]['auth']=='Bearer synthetic-key-not-real' and seen[-1]['custom']=='saved' and seen[-1]['cookie'] is None
        for wrong in [origin+'/gateway/v1/models',origin+'/gateway/v1/chat/completions?extra=1','http://127.0.0.1:1/gateway/v1/chat/completions']:
            assert relay(wrong)[0]==403
        assert relay(target,'unauthorized')[0]==401
        status,headers,_=relay(target,'limited');assert status==429 and headers['Retry-After']=='7'
        assert relay(target,'redirect')[0]==302
        try:relay(target,'truncated');raise AssertionError('Truncated response accepted')
        except __import__('http.client',fromlist=['IncompleteRead']).IncompleteRead:pass
        worker=threading.Thread(target=lambda:relay(target,'slow'));worker.start();assert slow.wait(5)
        begin=time.monotonic();invoke('_web_info',{});assert time.monotonic()-begin<0.8;worker.join()
        print('PASS: exact endpoint scope, server-side auth/custom headers, UTF-8 streaming, HTTP errors, rate-limit headers, redirects, truncated streams and concurrent UI RPC',flush=True)
        events=opener.open(base+'/_llmwiki/events?event=agent-event',timeout=20)
        assert events.readline().startswith(b': ready');events.readline()
        project=store('get','lastProject');run='transport-'+str(time.time_ns())
        invoke('agent_start_turn_stream',{'projectId':project['id'],'request':{'message':'transport-agent-proof','sessionId':'transport-session','runId':run,'mode':'fast','persistSession':True}})
        done=False
        while not done:
            line=events.readline()
            assert line,'Agent stream closed early'
            if line.startswith(b'data:'):
                event=json.loads(line[5:])
                if event['runId']==run:
                    payload=event['event'];assert payload['type']!='error',payload
                    done=payload['type']=='done'
        events.close()
        for _ in range(50):
            messages=invoke('agent_get_session',{'projectId':project['id'],'sessionId':'transport-session'})
            if any('Live Agent configuration confirmed.' in m['content'] for m in messages):break
            time.sleep(.1)
        else:raise AssertionError('Agent session was not persisted')
        print('PASS: original Agent uses the saved live model configuration and persists its response',flush=True)
        cfg.update(apiMode='anthropic_messages',customEndpoint=origin+'/anthropic')
        store('set','llmConfig',cfg);assert relay(origin+'/anthropic/v1/messages')[0]==200
        assert seen[-1]['key']=='synthetic-key-not-real' and seen[-1]['version']=='2023-06-01'
        stop();start();assert store('get','llmConfig')==cfg
        assert relay(origin+'/anthropic/v1/messages')[0]==200
        assert (args.workspace/'external-ui-state.json').stat().st_mode&0o777==0o600
        print('PASS: Anthropic wire/auth and saved model/credential persistence across restart; fixture mode never substituted',flush=True)
        subprocess.run(['python3',str(args.root/'verify_source.py')],check=True)
    finally:
        if process is not None:stop()
        model.shutdown();model.server_close()

if __name__=='__main__':main()
