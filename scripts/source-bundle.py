#!/usr/bin/env python3
"""Content-addressed local Guard sources; no registry publication or identity claim."""
import argparse, hashlib, io, json, pathlib, re, subprocess, tarfile
NAMES=('specguard','guardengine','gitguard')
LIMIT=64*1024*1024

def safe_name(name):
 p=pathlib.PurePosixPath(name)
 if not name or p.is_absolute() or any(x in ('..','.git','target') for x in p.parts) or '\\' in name or str(p)!=name:
  raise ValueError('unsafe archive path')
 return p

def pack(repos,destination):
 if set(repos)!=set(NAMES):raise ValueError('exact three source repositories required')
 files={};pins={};total=0
 for name in NAMES:
  repo,pin=repos[name]
  if not re.fullmatch('[0-9a-f]{40}',pin):raise ValueError('full immutable commit required')
  actual=subprocess.check_output(['git','-C',str(repo),'rev-parse',pin+'^{commit}'],text=True).strip()
  if actual!=pin:raise ValueError('source commit mismatch')
  selected=['Cargo.toml','Cargo.lock','src']
  if name=='specguard':selected+=['fixtures','tests/common','tests/producer_support','tests/compatibility_matrix.rs']
  available=subprocess.check_output(['git','-C',str(repo),'ls-tree','--name-only',pin],text=True).splitlines()
  selected=[p for p in selected if p.split('/')[0] in available]
  raw=subprocess.check_output(['git','-C',str(repo),'archive',pin,'--',*selected])
  if len(raw)>LIMIT:raise ValueError('archive byte limit')
  with tarfile.open(fileobj=io.BytesIO(raw)) as archive:
   for member in archive:
    if member.isdir():continue
    if not member.isfile():raise ValueError('nonregular source entry')
    safe_name(member.name)
    total+=member.size
    if total>LIMIT or len(files)>=8192:raise ValueError('source budget')
    files[f'sources/{name}/{member.name}']=archive.extractfile(member).read()
  pins[name]=pin
 manifest={'apiVersion':'specguard.source-bundle/v1','profile':'local-linux-source-development','pins':pins,'files':{n:hashlib.sha256(b).hexdigest() for n,b in sorted(files.items())},'productionRegistryRelease':False}
 files['MANIFEST.json']=json.dumps(manifest,sort_keys=True,indent=2).encode()+b'\n'
 stream=io.BytesIO()
 with tarfile.open(fileobj=stream,mode='w') as archive:
  for name,data in sorted(files.items()):
   info=tarfile.TarInfo(name);info.size=len(data);info.mode=0o644;info.mtime=0
   archive.addfile(info,io.BytesIO(data))
 data=stream.getvalue();digest=hashlib.sha256(data).hexdigest();destination=pathlib.Path(destination);destination.mkdir(parents=True,exist_ok=True);path=destination/(digest+'.tar')
 try:
  with path.open('xb') as out:out.write(data)
 except FileExistsError:
  if path.read_bytes()!=data:raise ValueError('immutable destination collision')
 return path

def unpack_checked(path,destination):
 path=pathlib.Path(path);destination=pathlib.Path(destination)
 if destination.exists() or destination.is_symlink():raise ValueError('destination must be new')
 if path.stat().st_size>LIMIT+8*1024*1024:raise ValueError('archive budget')
 raw=path.read_bytes()
 if path.stem!=hashlib.sha256(raw).hexdigest():raise ValueError('archive content address mismatch')
 files={};total=0
 try:
  with tarfile.open(fileobj=io.BytesIO(raw)) as archive:
   for member in archive:
    if not member.isfile():raise ValueError('only regular package files allowed')
    safe_name(member.name);total+=member.size
    if member.name in files or total>LIMIT or len(files)>=8192:raise ValueError('duplicate or oversized package')
    files[member.name]=archive.extractfile(member).read()
  manifest=json.loads(files.pop('MANIFEST.json'))
 except (tarfile.TarError,KeyError,json.JSONDecodeError) as error:raise ValueError('invalid package') from error
 if set(manifest)!={'apiVersion','profile','pins','files','productionRegistryRelease'} or manifest['apiVersion']!='specguard.source-bundle/v1' or manifest['profile']!='local-linux-source-development' or manifest['productionRegistryRelease'] is not False:raise ValueError('unknown package profile')
 if set(manifest['pins'])!=set(NAMES) or any(not re.fullmatch('[0-9a-f]{40}',pin) for pin in manifest['pins'].values()):raise ValueError('invalid source pins')
 if manifest['files']!={n:hashlib.sha256(b).hexdigest() for n,b in files.items()}:raise ValueError('package inventory/digest mismatch')
 for n in files:
  if len(safe_name(n).parts)<3 or safe_name(n).parts[0]!='sources' or safe_name(n).parts[1] not in NAMES:raise ValueError('unknown source namespace')
 destination.mkdir(mode=0o700)
 for name,data in files.items():
  out=destination/name;out.parent.mkdir(parents=True,exist_ok=True)
  with out.open('xb') as file:file.write(data)
 (destination/'MANIFEST.json').write_text(json.dumps(manifest,sort_keys=True,indent=2)+'\n')
 return manifest

if __name__=='__main__':
 parser=argparse.ArgumentParser(description=__doc__);sub=parser.add_subparsers(dest='command',required=True)
 packer=sub.add_parser('pack');packer.add_argument('destination')
 for name in NAMES:packer.add_argument('--'+name,nargs=2,metavar=('REPOSITORY','COMMIT'),required=True)
 unpacker=sub.add_parser('unpack');unpacker.add_argument('archive');unpacker.add_argument('destination')
 args=parser.parse_args()
 if args.command=='pack':print(pack({name:getattr(args,name) for name in NAMES},args.destination))
 else:print(json.dumps(unpack_checked(args.archive,args.destination),sort_keys=True))
