#!/usr/bin/env python3
"""Fresh-case exact-pin native research only; never production lifecycle authority."""
import os,sys,json,hashlib,pathlib,tempfile,pwd,stat,re,time,subprocess,selectors,signal,threading,argparse
BASE=None
PINS={'2.1.308':('claude-2.1.308-darwin-arm64',274675680,'ca923c86ac84b30574a82331a05b6e066c27477a926ceaffe46a845a33e4dd37'),'2.1.314':('claude-2.1.314-darwin-arm64',278848096,'21d6e7b9a6f07b4cfcb7df13760b3b72aa25e084e13a5354413382987ba832ed')}
LIMIT=262144
REAL=pathlib.Path(pwd.getpwuid(os.getuid()).pw_dir).resolve()
ROOT=None
POLICY=None
def digest(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1048576),b''):h.update(b)
 return h.hexdigest()
def checkexe(version):
 name,size,sha=PINS[version]; p=BASE/name
 if BASE.parent!=pathlib.Path('/private/tmp') or not BASE.name.startswith('agentmender-'):raise RuntimeError('artifact directory must be private AgentMender-owned temporary root')
 for a in [p,*p.parents]:
  if a.is_symlink():raise RuntimeError('symlink ancestor')
 s=p.lstat(); parent=BASE.stat()
 if not stat.S_ISREG(s.st_mode) or s.st_nlink!=1 or s.st_uid!=os.getuid() or s.st_mode&0o222 or parent.st_uid!=os.getuid() or parent.st_mode&0o077 or s.st_size!=size or digest(p)!=sha:raise RuntimeError('exact artifact preflight failed')
 return p
def q(p):
 value=str(p)
 if not value.isascii() or any(ord(ch)<32 or ord(ch)==127 for ch in value):raise ValueError('unsupported Seatbelt path literal')
 return json.dumps(value)

def env(c):return {'HOME':str(c/'home'),'CLAUDE_CONFIG_DIR':str(c/'config'),'TMPDIR':str(c/'tmp'),'CLAUDE_CODE_TMPDIR':str(c/'tmp'),'PATH':'/usr/bin:/bin','DISABLE_AUTOUPDATER':'1','CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC':'1','TERM':'dumb','NO_COLOR':'1'}
def snap(root):
 out={}
 def walk(p):
  for a in sorted(p.iterdir()):
   s=a.lstat(); d={'mode':stat.S_IMODE(s.st_mode),'mtime_ns':s.st_mtime_ns,'size':s.st_size}
   if stat.S_ISLNK(s.st_mode):d.update(type='symlink',target=os.readlink(a))
   elif stat.S_ISDIR(s.st_mode):d['type']='dir'
   elif stat.S_ISREG(s.st_mode):d.update(type='file',sha256=digest(a))
   else:d['type']='other'
   out[str(a.relative_to(root))]=d
   if d['type']=='dir':walk(a)
 walk(root);return out
def bounded(argv,cwd,environ,interrupt=None,watch=None,timeout=10):
 p=None;sel=selectors.DefaultSelector();start=time.monotonic();streams={'stdout':bytearray(),'stderr':bytearray()};cause=None;sent=False
 try:
  p=subprocess.Popen(argv,cwd=cwd,env=environ,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
  for n in streams:
   os.set_blocking(getattr(p,n).fileno(),False);sel.register(getattr(p,n),selectors.EVENT_READ,n)
  while sel.get_map():
   elapsed=time.monotonic()-start
   if interrupt and not sent and ((watch is not None and not watch.exists()) or (watch is None and elapsed>=interrupt[0])) and p.poll() is None:
    p.send_signal(interrupt[1]);sent=True
   if elapsed>=timeout:cause='timeout';break
   for slot,_ in sel.select(0.002 if interrupt else 0.02):
    b=os.read(slot.fileobj.fileno(),65536)
    if not b:sel.unregister(slot.fileobj);continue
    if sum(map(len,streams.values()))+len(b)>LIMIT:cause='output-limit';break
    streams[slot.data].extend(b)
   if cause:break
  if cause and p.poll() is None:p.kill()
  try:p.wait(timeout=max(.1,timeout-(time.monotonic()-start)))
  except subprocess.TimeoutExpired:cause='timeout';p.kill();p.wait()
  return {'returncode':p.returncode,'bounded_failure':cause,'elapsed_seconds':round(time.monotonic()-start,6),'interruption_sent':sent,**{n:bytes(b).decode('utf8','replace') for n,b in streams.items()}}
 finally:
  if p is not None:
   if p.poll() is None:p.kill()
   p.wait()
   for n in streams:
    pipe=getattr(p,n,None)
    if pipe is not None:pipe.close()
  sel.close()
def put(c,name,value='synthetic retained marker\n'):
 p=c/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(value);return p
def key(p):return re.sub('[^a-zA-Z0-9]','-',str(p))
def sid(n):return f'{n:08d}-{n:04d}-4{n:03d}-8{n:03d}-{n:012d}'
def fixture(label,shape='basic',empty=False,large=False):
 c=ROOT/label;c.mkdir(mode=0o700)
 for d in ['home','config','cwd','tmp']:(c/d).mkdir(mode=0o700)
 a=c/'cwd'/('a-b' if shape=='collision' else 'target')
 b=c/'cwd'/'a'/'b' if shape=='collision' else (a/'nested' if shape=='nested' else c/'cwd'/'other')
 if shape=='case':a=c/'cwd'/'CaseTarget';b=c/'cwd'/'other'
 if shape=='unicode':a=c/'cwd'/'caf\u00e9';b=c/'cwd'/'other'
 paths=[a,b,c/'cwd'/'retained',c/'cwd'/(a.name+'-prefix')]
 if empty:return c,a,paths
 project_map={};history=[]
 for n,p in enumerate(paths,1):
  p.mkdir(parents=True,exist_ok=True);s=sid(n);k=key(p)
  put(c,f'config/projects/{k}/{s}.jsonl',json.dumps({'type':'user','sessionId':s,'cwd':str(p),'version':label.split('-',1)[0],'message':{'role':'user','content':'synthetic disposable marker'}})+'\n')
  for name in [f'config/tasks/{s}/1.json',f'config/debug/{s}.txt',f'config/file-history/{s}/snapshot',f'config/plans/{s}.md',f'config/paste-cache/{s}',f'config/image-cache/{s}/image',f'config/uploads/{s}/attachment',f'config/dev-mods/{s}/mod',f'config/session-env/{s}.json',f'config/sessions/{s}.json',f'config/todos/{s}.json',f'config/usage-data/{s}.json',f'config/feedback/drafts/{s}.json',f'config/jobs/{s}.json',f'config/daemon/{s}.json',f'tmp/claude-{os.getuid()}/{k}/{s}/scratchpad/marker',f'tmp/{s}/images/marker',f'config/projects/{k}/{s}/subagents/agent-test.jsonl',f'config/projects/{k}/{s}/tool-results/marker',f'config/projects/{k}/{s}/mcp-tasks/marker',f'config/projects/{k}/memory/MEMORY.md']:put(c,name)
  put(c,str(p.relative_to(c))+'/source.txt')
  project_map[str(p)]={'hasTrustDialogAccepted':True};history.append({'project':str(p),'sessionId':s,'display':'synthetic'})
 put(c,'config/.claude.json',json.dumps({'projects':project_map}));put(c,'home/.claude.json',json.dumps({'projects':project_map}));put(c,'config/history.jsonl','\n'.join(map(json.dumps,history))+'\n')
 for name in ['config/shell-snapshots/retained.sh','config/backups/retained.json','config/stats-cache.json','config/settings.json','config/remote-settings.json','config/agent-memory/marker','config/plugins/marker','config/skills/marker','config/cache/marker','config/feedback-bundles/marker','config/global-marker']:put(c,name,'{}\n' if name.endswith('.json') else 'synthetic retained marker\n')
 if large:
  for i in range(2000):put(c,f'config/tasks/{sid(1)}/{i+2}.json')
 return c,a,paths
OBS=[]
def probe(version,label,shape='basic',flag='--yes',selector=None,denial=None,command=None,empty=False,nonbare=False,interrupt=None,watch_first=False,large=False,modify=None,environment_override=None,timeout=10,expected_timeout=False):
 c,a,paths=fixture(version+'-'+label,shape,empty,large)
 if modify:modify(c,a,paths)
 target=selector(c,a,paths) if selector else str(a)
 args=command if command else ([] if nonbare else ['--bare'])+['purge',target,flag]
 extra='' if denial is None else '(deny file-write* (subpath '+q(denial(c,a,paths))+'))'
 sb=ROOT/(version+'-'+label+'.sb');sb.write_text(POLICY+'(deny file-write* (require-not (subpath '+q(c)+')))'+extra)
 before=snap(c);exe=checkexe(version)
 environ=env(c)
 if environment_override:environ.update(environment_override)
 r=bounded(['/usr/bin/sandbox-exec','-f',str(sb),str(exe),*args],c/'cwd',environ,interrupt,c/f'config/tasks/{sid(1)}/1.json' if watch_first else None,timeout)
 after=snap(c)
 removed=sorted(before.keys()-after.keys());created=sorted(after.keys()-before.keys());changed=sorted(k for k in before.keys()&after.keys() if before[k]!=after[k])
 r.update(label=label,version=version,args=args,case=str(c),fresh_case=True,before=before,after=after,removed=removed,created=created,changed=changed,removed_files=[k for k in removed if before[k]['type']=='file'],created_files=[k for k in created if after[k]['type']=='file'],changed_files=[k for k in changed if before[k]['type']=='file' and after[k]['type']=='file'])
 r['expected_timeout']=expected_timeout
 (ROOT/(version+'-'+label+'.json')).write_text(json.dumps(r,indent=2));OBS.append(r)
 if r['bounded_failure'] and not (expected_timeout and r['bounded_failure']=='timeout'):raise RuntimeError('unexpected bounded failure: '+label)
 if expected_timeout and r['bounded_failure']!='timeout':raise RuntimeError('forced timeout not observed')
 print(json.dumps({k:r[k] for k in ['version','label','returncode','bounded_failure','interruption_sent','removed_files','created_files','changed_files']}),flush=True)
 return r

def attest():
 outside=pathlib.Path(tempfile.mkdtemp(prefix='agentmender-m9-fresh-canary-',dir='/private/tmp'));os.chmod(outside,0o700);canary=outside/'marker';canary.write_text('ownedwritecanary');prior=canary.stat().st_mtime_ns
 home=pathlib.Path(tempfile.mkdtemp(prefix='.agentmender-owned-read-canary-',dir=REAL));os.chmod(home,0o700);secret=home/'marker';secret.write_text('owned-read-canary-nonsecret')
 try:
  write=bounded(['/usr/bin/sandbox-exec','-f',str(ROOT/'sandbox.sb'),'/usr/bin/touch',str(canary)],ROOT,{})
  read=bounded(['/usr/bin/sandbox-exec','-f',str(ROOT/'sandbox.sb'),'/bin/cat',str(secret)],ROOT,{})
  fork=bounded(['/usr/bin/sandbox-exec','-f',str(ROOT/'sandbox.sb'),'/usr/bin/perl','-e','my $p=fork(); if(!defined($p)){print "DENIED\\n"; exit 7;} exit 0;'],ROOT,{})
  network=bounded(['/usr/bin/sandbox-exec','-f',str(ROOT/'sandbox.sb'),'/usr/bin/perl','-MSocket','-e','socket(my $s,PF_INET,SOCK_STREAM,0) or die "socket denied: $!"; connect($s,sockaddr_in(9,inet_aton("127.0.0.1"))) or die "connect denied: $!";'],ROOT,{})
  report={'outside_write':write,'realhome_read':read,'fork':fork,'network':network,'outside_unchanged':canary.stat().st_mtime_ns==prior}
  (ROOT/'isolation-attestation.json').write_text(json.dumps(report,indent=2))
  if (write['returncode']==0 or read['returncode']==0 or fork['returncode']!=7 or fork['stdout']!='DENIED\n' or network['returncode']==0 or any(r['bounded_failure'] for r in [write,read,fork,network]) or not all('Operation not permitted' in r['stderr'] for r in [write,read,network]) or not report['outside_unchanged']):raise RuntimeError('isolation attestation failed')
 finally:secret.unlink();home.rmdir();canary.unlink();outside.rmdir()

def verify_core(v):
 by={o['label']:o for o in OBS};dry=by['basic-dry'];collision=by['collision-apply'];nested=by['nested-apply']
 checks={'normal_cases_exit0':all(by[l]['returncode']==0 for l in ['basic-dry','basic-apply','collision-apply','nested-apply']),'dry_run_config_write':'config/.claude.json' in dry['changed_files'],'dry_run_backup_creation':any(p.startswith('config/backups/') for p in dry['created_files']),'selector_collision':sum(p.endswith('.jsonl') and '/subagents/' not in p for p in collision['removed_files'])==2,'nested_cascade':sum(p.endswith('.jsonl') and '/subagents/' not in p for p in nested['removed_files'])==2,'first_denial_exit1_partial':by['denied-first']['returncode']==1 and len(by['denied-first']['removed_files'])>0,'late_denial_exit1_partial':by['denied-late']['returncode']==1 and len(by['denied-late']['removed_files'])>0}
 (ROOT/'core-findings-checks.json').write_text(json.dumps(checks,indent=2))
 if not all(checks.values()):raise RuntimeError('exact-version core evidence changed; review required')
def supplement(v):
 probe(v,'forced-timeout-100ms',timeout=.1,expected_timeout=True)
 for cp in ['\u2028','\u2029']:
  def unicode_records(c,a,p,cp=cp):
   nested=a/('child'+cp+'project');nested.mkdir(parents=True)
   k=key(nested);s=sid(9)
   put(c,f'config/projects/{k}/{s}.jsonl',json.dumps({'type':'user','sessionId':s,'cwd':str(nested),'version':v},ensure_ascii=False)+'\n')
   cfg=json.loads((c/'config/.claude.json').read_text());cfg['projects'][str(nested)]={'hasTrustDialogAccepted':True};put(c,'config/.claude.json',json.dumps(cfg,ensure_ascii=False))
   put(c,'config/history.jsonl',json.dumps({'project':str(a),'sessionId':sid(1),'display':'synthetic'+cp+'prompt'},ensure_ascii=False)+'\n'+json.dumps({'project':str(nested),'sessionId':s,'display':'synthetic nested'},ensure_ascii=False)+'\n'+json.dumps({'project':str(p[2]),'sessionId':sid(3),'display':'retained'},ensure_ascii=False)+'\n')
  probe(v,'unicode-separator-'+hex(ord(cp)),modify=unicode_records)
 def custom(c,a,p):
  original=c/'config/projects'/key(a);original.rename(c/'config/projects'/'fixed-synthetic-project-name')
 probe(v,'custom-project-dir',modify=custom,environment_override={'CLAUDE_CODE_PROJECT_DIR_NAME':'fixed-synthetic-project-name'})
 def longpaths(c,a,p):
  # Paths are valid directories; only the transcript bucket prefix is >200 chars.
  pa=c/'cwd'/('a'*90)/('b'*90)/'suffix-a';pb=pa.parent/'suffix-b'
  for n,path in [(7,pa),(8,pb)]:
   path.mkdir(parents=True,exist_ok=True)
   # Legacy >200 key must remain bounded to the native 200-character candidate.
   bucket=key(path)[:200];put(c,f'config/projects/{bucket}/{sid(n)}.jsonl',json.dumps({'type':'user','sessionId':sid(n),'cwd':str(path),'version':v})+'\n')
  cfg=json.loads((c/'config/.claude.json').read_text());cfg['projects'].update({str(pa):{},str(pb):{}});put(c,'config/.claude.json',json.dumps(cfg))
 probe(v,'long-shared-prefix',modify=longpaths,selector=lambda c,a,p:str(c/'cwd'/('a'*90)/('b'*90)/'suffix-a'))
 def hashed_longpaths(c,a,p):
  pa=c/'cwd'/('a'*90)/('b'*90)/'suffix-a';pb=pa.parent/'suffix-b'
  for n,path in [(7,pa),(8,pb)]:
   path.mkdir(parents=True,exist_ok=True);h=0
   for char in str(path):h=((h<<5)-h+ord(char))&0xffffffff
   h=abs(h if h<0x80000000 else h-0x100000000);digits='0123456789abcdefghijklmnopqrstuvwxyz';suffix=''
   while h:suffix=digits[h%36]+suffix;h//=36
   bucket=key(path)[:200]+'-'+(suffix or '0')
   put(c,f'config/projects/{bucket}/{sid(n)}.jsonl',json.dumps({'type':'user','sessionId':sid(n),'cwd':str(path),'version':v})+'\n')
  cfg=json.loads((c/'config/.claude.json').read_text());cfg['projects'].update({str(pa):{},str(pb):{}});put(c,'config/.claude.json',json.dumps(cfg))
 probe(v,'long-sdk-hashed-prefix',modify=hashed_longpaths,selector=lambda c,a,p:str(c/'cwd'/('a'*90)/('b'*90)/'suffix-a'))
def main():
 global ROOT,POLICY,BASE
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--allow-isolated-native',action='store_true');parser.add_argument('--artifact-root',type=pathlib.Path,required=True);parser.add_argument('--version',choices=sorted(PINS),default='2.1.314');mode=parser.add_mutually_exclusive_group();mode.add_argument('--core-only',action='store_true');mode.add_argument('--supplement-only',action='store_true');args=parser.parse_args()
 if not args.allow_isolated_native or sys.platform!='darwin':parser.error('explicit opt-in and macOS required')
 BASE=args.artifact_root
 if not BASE.is_absolute() or BASE.resolve()!=BASE:parser.error('artifact directory must be canonical absolute path')
 v=args.version;q(REAL);checkexe(v)
 ROOT=pathlib.Path(tempfile.mkdtemp(prefix='agentmender-m9-current-fresh-',dir='/private/tmp')).resolve();os.chmod(ROOT,0o700)
 POLICY='(version 1)(allow default)(deny file-write* (require-not (subpath '+q(ROOT)+')))(deny file-read-data (subpath '+q(REAL)+'))(deny network*)(deny process-fork)(deny mach-lookup)'
 (ROOT/'sandbox.sb').write_text(POLICY)
 print('ROOT='+str(ROOT),flush=True);attest()
 if args.supplement_only:
  supplement(v);finish(v);return
 for label,metadata_args in [('version',['--version']),('help',['--help']),('purge-help',['purge','--help']),('project-help',['project','--help'])]:probe(v,label,command=metadata_args,empty=True)
 if any(o['returncode']!=0 for o in OBS):raise RuntimeError('metadata help/version failed')
 for label,shape,flag,denial in [('basic-dry','basic','--dry-run',None),('basic-apply','basic','--yes',None),('collision-apply','collision','--yes',None),('nested-apply','nested','--yes',None),('denied-first','basic','--yes',lambda c,a,p:c/f'config/tasks/{sid(1)}'),('denied-late','basic','--yes',lambda c,a,p:c/'config/projects'/key(a))]:probe(v,label,shape,flag,denial=denial)
 if OBS[0]['stdout'].strip()!=v+' (Claude Code)':raise RuntimeError('native exact version mismatch')
 verify_core(v)
 (ROOT/'baseline-six-complete').write_text('fresh independent cases complete\n')
 for label,denial in [('nested-denied-first',lambda c,a,p:c/f'config/tasks/{sid(1)}'),('nested-denied-late',lambda c,a,p:c/'config/projects'/key(a))]:probe(v,label,shape='nested',denial=denial)
 if args.core_only:finish(v);return
 for label,selector,shape in [('normalize-dot',lambda c,a,p:str(a)+'/./','basic'),('normalize-parent',lambda c,a,p:str(a)+'/../target','basic'),('relative',lambda c,a,p:'target','basic'),('trailing-slash',lambda c,a,p:str(a)+'/','basic'),('case-alias',lambda c,a,p:str(a).replace('CaseTarget','casetarget'),'case'),('unicode-decomposed',lambda c,a,p:str(a).replace('caf\u00e9','cafe\u0301'),'unicode')]:probe(v,label,shape,selector=selector)
 def symlink(c,a,p):(c/'cwd'/'alias').symlink_to(a,target_is_directory=True)
 probe(v,'symlink-selector',selector=lambda c,a,p:str(c/'cwd'/'alias'),modify=symlink)
 probe(v,'no-state-apply',empty=True);probe(v,'no-state-dry',flag='--dry-run',empty=True)
 probe(v,'nonbare-startup',nonbare=True);probe(v,'no-state-nonbare',nonbare=True,empty=True)
 for label,denial in [('denied-config',lambda c,a,p:c/'config/.claude.json'),('denied-history',lambda c,a,p:c/'config/history.jsonl'),('deny-all-writes',lambda c,a,p:c)]:probe(v,label,denial=denial)
 def no_config(c,a,p):
  for f in ['config/.claude.json','home/.claude.json']:put(c,f,'{"projects":{}}')
 probe(v,'encoded-only',modify=no_config)
 def mismatch(c,a,p):
  f=c/f'config/projects/{key(a)}/{sid(1)}.jsonl';data=json.loads(f.read_text());data['cwd']=str(p[1]);f.write_text(json.dumps(data)+'\n')
 probe(v,'transcript-cwd-mismatch',modify=mismatch)
 def orphan(c,a,p):
  put(c,'config/projects/orphan-native-key/'+sid(9)+'.jsonl',json.dumps({'type':'user','sessionId':sid(9),'cwd':str(a),'version':v})+'\n')
 probe(v,'unrelated-key-same-cwd',modify=orphan)
 for delay in [0.001,0.1,0.25]:probe(v,'sigterm-'+str(delay),interrupt=(delay,signal.SIGTERM))
 probe(v,'sigkill-first-removal',large=True,interrupt=(0,signal.SIGKILL),watch_first=True)
 # Deliberate same-new-fixture concurrency; neither process has any other case state.
 c,a,_=fixture(v+'-concurrent');before=snap(c);exe=checkexe(v);results=[None,None];errors=[]
 concurrent_policy=ROOT/(v+'-concurrent.sb');concurrent_policy.write_text(POLICY+'(deny file-write* (require-not (subpath '+q(c)+')))')
 def run(i):
  try:results[i]=bounded(['/usr/bin/sandbox-exec','-f',str(concurrent_policy),str(exe),'--bare','purge',str(a),'--yes'],c/'cwd',env(c))
  except BaseException as error:errors.append(repr(error))
 ts=[threading.Thread(target=run,args=(i,)) for i in range(2)]
 for t in ts:t.start()
 for t in ts:t.join()
 if errors or any(r is None or r['bounded_failure'] for r in results):raise RuntimeError('concurrency failures: '+repr(errors))
 after=snap(c);(ROOT/(v+'-concurrent.json')).write_text(json.dumps({'fresh_case':True,'deliberately_shared_process_count':2,'before':before,'after':after,'results':results},indent=2))
 supplement(v)
 finish(v,2)
def finish(v,concurrent=0):
 checkexe(v)
 summary={'root':str(ROOT),'version':v,'native_writes_restricted_to_case':True,'native_probe_count':len(OBS)+concurrent,'sequential_probe_count':len(OBS),'concurrent_probes':concurrent,'yes_invocations':sum('--yes' in o['args'] for o in OBS)+concurrent,'production_execution_supported':False,'fresh_case_per_invocation':True,'concurrency_exception':'two processes deliberately share one fresh case' if concurrent else None,'binary_sha256':PINS[v][2],'source_binary_unchanged':True,'labels':[o['label'] for o in OBS]}
 (ROOT/'summary.json').write_text(json.dumps(summary,indent=2));print(json.dumps(summary),flush=True)
if __name__=='__main__':main()
