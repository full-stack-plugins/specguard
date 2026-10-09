import importlib.util, pathlib, tempfile, unittest, subprocess, io, tarfile, hashlib, json
ROOT=pathlib.Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('bundle',ROOT/'scripts/source-bundle.py')
bundle=importlib.util.module_from_spec(spec);spec.loader.exec_module(bundle)
class BundleTests(unittest.TestCase):
 def test_real_git_archive_roundtrip_and_tamper(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=pathlib.Path(tmp);repos={}
   for name in ['specguard','guardengine','gitguard']:
    repo=root/name;repo.mkdir();(repo/'src').mkdir();(repo/'src/lib.rs').write_text('pub fn real() {}\n');(repo/'Cargo.toml').write_text('[package]\nname="'+name+'"\nversion="0.1.0"\n');(repo/'Cargo.lock').write_text('version=4\n')
    subprocess.run(['git','init','-q',str(repo)],check=True)
    subprocess.run(['git','-C',str(repo),'add','.'],check=True)
    subprocess.run(['git','-C',str(repo),'-c','user.name=fixture','-c','user.email=fixture@invalid','commit','-qm','source'],check=True)
    pin=subprocess.check_output(['git','-C',str(repo),'rev-parse','HEAD'],text=True).strip();repos[name]=(repo,pin)
   package=bundle.pack(repos,root/'artifacts');manifest=bundle.unpack_checked(package,root/'consumer')
   self.assertEqual(manifest['pins']['specguard'],repos['specguard'][1]);self.assertEqual((root/'consumer/sources/specguard/src/lib.rs').read_text(),'pub fn real() {}\n')
   with self.assertRaises(ValueError):bundle.unpack_checked(package,root/'consumer')
   damaged=root/'damaged.tar';damaged.write_bytes(package.read_bytes()[:-2000]+b'bad')
   with self.assertRaises(ValueError):bundle.unpack_checked(damaged,root/'bad')
 def test_rehashed_unsafe_packages_fail_before_destination_creation(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=pathlib.Path(tmp)
   manifest={'apiVersion':'specguard.source-bundle/v1','profile':'local-linux-source-development','pins':{n:'a'*40 for n in bundle.NAMES},'files':{},'productionRegistryRelease':False}
   cases=[]
   for path in ['../escape','/absolute','sources/specguard/target/file','sources/specguard/.git/config']:
    cases.append([(path,b'x',None)])
   cases += [[('sources/specguard/link',b'',tarfile.SYMTYPE)], [('same',b'a',None),('same',b'b',None)]]
   for field,value in [('apiVersion','future'),('profile','production'),('productionRegistryRelease',True),('pins',{})]:
    altered=dict(manifest);altered[field]=value
    cases.append([('MANIFEST.json',json.dumps(altered).encode(),None)])
   altered=dict(manifest);altered['files']={'sources/specguard/file':'0'*64}
   cases.append([('MANIFEST.json',json.dumps(altered).encode(),None),('sources/specguard/file',b'tampered',None)])
   for i,entries in enumerate(cases):
    stream=io.BytesIO()
    with tarfile.open(fileobj=stream,mode='w') as archive:
     for name,data,kind in entries:
      info=tarfile.TarInfo(name);info.size=len(data)
      if kind:info.type=kind;info.linkname='/etc/passwd'
      archive.addfile(info,io.BytesIO(data))
    raw=stream.getvalue();path=root/(hashlib.sha256(raw).hexdigest()+'.tar');path.write_bytes(raw)
    destination=root/f'out{i}'
    with self.assertRaises(ValueError):bundle.unpack_checked(path,destination)
    self.assertFalse(destination.exists())
if __name__=='__main__':unittest.main()
