#!/usr/bin/env python3
"""Inspect an offline scratch target, including synthetic exclusion sentinels."""
import argparse,hashlib,subprocess,tarfile
from pathlib import Path

def _check(context,image):
    context=Path(context).resolve()
    sentinels={'.env','probe-test-cookie','fixtures/should-not-build.txt','node_modules/should-not-build.txt','backend/private.env','upstream/private-note.env'}
    for name in sentinels:
        file=context/name
        if file.exists() or file.is_symlink():raise ValueError('Refusing to overwrite sentinel path: '+name)
        file.parent.mkdir(parents=True,exist_ok=True)
        file.write_text('Synthetic context exclusion sentinel; no real secret.\n')
    original=hashlib.file_digest((context/'upstream-source.tar').open('rb'),'sha256').hexdigest()
    subprocess.run(['docker','build','--network','none','--target','context-check','-t',image,str(context)],check=True)
    container=subprocess.check_output(['docker','create',image,'/not-executed'],text=True).strip()
    try:
        exporter=subprocess.Popen(['docker','export',container],stdout=subprocess.PIPE)
        present=set();archive_digest=None
        with tarfile.open(fileobj=exporter.stdout,mode='r|') as archive:
            for item in archive:
                if item.isfile():present.add(item.name)
                if item.name=='context/upstream-source.tar':archive_digest=hashlib.file_digest(archive.extractfile(item),'sha256').hexdigest()
        exporter.stdout.read();exporter.stdout.close()
        assert exporter.wait(timeout=30)==0
        assert {'context/upstream-source.tar','context/package.json','context/backend/main.rs'}<=present
        assert not any('context/'+name in present for name in sentinels),present
        assert not any(name.startswith(('context/fixtures/','context/node_modules/','context/upstream/')) for name in present)
        assert archive_digest==original
        print('PASS: only allowed context inputs packaged; synthetic cookies/env/data/cache/untracked upstream files excluded; pinned source archive unchanged')
    finally:subprocess.run(['docker','rm',container],check=True,stdout=subprocess.DEVNULL)

def check(context,image):
    context=Path(context).resolve()
    names=['.env','probe-test-cookie','fixtures/should-not-build.txt','node_modules/should-not-build.txt','backend/private.env','upstream/private-note.env']
    marker=b'Synthetic context exclusion sentinel; no real secret.\n'
    for name in names:
        file=context/name
        if any(parent.is_symlink() for parent in [file,*file.parents] if parent!=context):
            raise ValueError('Refusing symlink sentinel path: '+name)
        if file.exists():raise ValueError('Refusing existing sentinel path: '+name)
    try:
        _check(context,image)
    finally:
        for name in names:
            file=context/name
            if file.is_file() and not file.is_symlink() and file.read_bytes()==marker:
                file.unlink()

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('context');parser.add_argument('--image',default='llmwiki-unchanged-context-check:local')
    args=parser.parse_args();check(args.context,args.image)
