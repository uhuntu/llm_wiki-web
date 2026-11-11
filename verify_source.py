#!/usr/bin/env python3
"""Verify every pinned Git blob without requiring the original repository."""
import hashlib,json,os
from pathlib import Path

def verify(root):
    root=Path(root);manifest=json.loads((root/'upstream-manifest.json').read_text())
    for item in manifest['files']:
        file=root/'upstream'/item['path']
        data=os.readlink(file).encode() if item['mode']=='120000' else file.read_bytes()
        digest=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
        if digest!=item['gitBlob']:raise ValueError('Upstream source changed: '+item['path'])
    return len(manifest['files'])
if __name__=='__main__':print(f'PASS: {verify(Path(__file__).resolve().parent)} original files unchanged')
