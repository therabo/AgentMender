#!/usr/bin/env python3
"""Source-only extraction. Never imports/starts Electron or reads provider data.
Writes only under the caller's fresh owned output directory.
"""
import argparse,hashlib,json,plistlib,sqlite3,struct,subprocess
from pathlib import Path

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('bundle', type=Path)
parser.add_argument('output', type=Path)
args=parser.parse_args()
OUT=args.output
APP=args.bundle
ASAR=APP/'Contents/Resources/app.asar'
ASSET='.vite/build/early-bootstrap-339-9edb60e5d8b0547e.js'
CATALOG_ASSET='.vite/build/early-bootstrap-439-a0f25ba32c465de1.js'
EXPECTED_ASAR='52e0a65f9fea51f4287bbd98c63343761f9424517bb793652bb554764bc0bcc8'
EXPECTED_ASSET='480cf9c01ab1794c1b788aaf1bc159e96972f341e7f7ec03a53a8769e7b0d175'

def sha(b):return hashlib.sha256(b).hexdigest()
def dump(name,obj):
 data=(json.dumps(obj,ensure_ascii=False,indent=2)+'\n').encode();(OUT/name).write_bytes(data);return sha(data)
def read_asset(path):
 with ASAR.open('rb') as f:
  sizes=struct.unpack('<4I',f.read(16));entry=json.loads(f.read(sizes[3]))
  for part in path.split('/'):entry=entry['files'][part]
  assert not entry.get('unpacked'),path
  f.seek(8+sizes[1]+int(entry['offset']));return f.read(entry['size'])
def canonical(obj):return json.dumps(obj,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()
def rows(db,sql):
 cur=db.execute(sql);cols=[x[0] for x in cur.description];return [dict(zip(cols,x)) for x in cur.fetchall()]

if not OUT.is_absolute() or OUT.resolve()!=OUT or not OUT.is_dir() or any(OUT.iterdir()):
 parser.error('output must be a fresh, empty, absolute canonical directory')
assert sha(ASAR.read_bytes())==EXPECTED_ASAR
info=plistlib.loads((APP/'Contents/Info.plist').read_bytes())
asset=read_asset(ASSET);assert sha(asset)==EXPECTED_ASSET
source=asset.decode('utf8')
start=source.index('let o={id:"20260902130908_create_live_visualization_suggestions"')
end=source.index(',W="codex_schema_migrations"',start)
fragment=source[start:end]
(OUT/'migration-declarations.extracted.js').write_text(fragment,encoding='utf8')
# Only declarations, schema data, and crypto checksum computation execute inside
# a fresh VM context. No require/process/fs/Electron objects are exposed there.
node_program='''const vm=require('node:vm'),crypto=require('node:crypto');
let input='';process.stdin.setEncoding('utf8');process.stdin.on('data',x=>input+=x);
process.stdin.on('end',()=>{const context=vm.createContext({s:{createHash:crypto.createHash}});
const result=new vm.Script(input+'; JSON.stringify(K)').runInContext(context,{timeout:1000});
process.stdout.write(result);});'''
(OUT/'evaluate-declarations.js').write_text(node_program,encoding='utf8')
result=subprocess.run(['node',str(OUT/'evaluate-declarations.js')],input=fragment,text=True,capture_output=True,check=True)
migrations=json.loads(result.stdout);assert len(migrations)==36
assert migrations[-1]['migration']['id']=='20261028065000_create_active_chat_turns'
# Preserve the native JSON serialization used to compute checksums as evidence.
(OUT/'ordered-migrations.native.json').write_text(result.stdout+'\n',encoding='utf8')
dump('ordered-migrations.json',{'evidence_class':'source_derived_not_runtime','migration_count':len(migrations),'migrations':migrations})

schema_start=source.index('CREATE TABLE IF NOT EXISTS ${W}',end)
schema_end=source.index('`',schema_start)
migration_schema=source[schema_start:schema_end].replace('${W}','codex_schema_migrations')
(OUT/'migration-ledger-create.sql').write_text(migration_schema+';\n',encoding='utf8')
db=sqlite3.connect(':memory:');db.execute(migration_schema)
legacy_versions={'legacy_0032_expand_local_thread_catalog_for_projects':32,'legacy_0034_create_local_thread_catalog_scan_tables':34}
for entry in migrations:
 for op in entry['migration']['operations']:
  kind=op['kind']
  if kind=='exec':db.executescript(op['sql'])
  elif kind=='run':db.execute(op['sql'],op['parameters'])
  elif kind=='addColumnIfMissing':
   table,column=op['table'],op['column']
   assert table.replace('_','').isalnum() and column.replace('_','').isalnum()
   if column not in [x['name'] for x in rows(db,'PRAGMA table_info("'+table+'")')]:
    db.execute('ALTER TABLE '+table+' ADD COLUMN '+column+' '+op['columnDefinition'])
  elif kind=='execIfColumnMissing':
   if op['column'] not in [x['name'] for x in rows(db,'PRAGMA table_info("'+op['table']+'")')]:db.executescript(op['sql'])
  else:raise ValueError(kind)
 version=legacy_versions.get(entry['migration']['id'])
 if version is not None:db.execute('PRAGMA user_version = '+str(version))
master=rows(db,'SELECT type,name,tbl_name,rootpage,sql FROM sqlite_master ORDER BY type,name')
# rootpage is included as observed disposable-derived information, not a runtime
# admission identity: page allocation is not schema semantics.
tables={}
for row in master:
 if row['type']!='table':continue
 name=row['name'];q='"'+name.replace('"','""')+'"'
 tables[name]={'table_info':rows(db,'PRAGMA table_info('+q+')'),'table_xinfo':rows(db,'PRAGMA table_xinfo('+q+')'),'foreign_key_list':rows(db,'PRAGMA foreign_key_list('+q+')'),'index_list':rows(db,'PRAGMA index_list('+q+')')}
indexes={}
for row in master:
 if row['type']!='index':continue
 name=row['name'];q='"'+name.replace('"','""')+'"'
 indexes[name]={'index_info':rows(db,'PRAGMA index_info('+q+')'),'index_xinfo':rows(db,'PRAGMA index_xinfo('+q+')')}
semantic={'sqlite_master':[{k:v for k,v in r.items() if k!='rootpage'} for r in master], 'tables':tables,'indexes':indexes,'user_version':db.execute('PRAGMA user_version').fetchone()[0],'application_id':db.execute('PRAGMA application_id').fetchone()[0]}
assert semantic['user_version']==34
assert len(tables)==14
assert not [r for r in master if r['type']=='trigger']
assert all(not t['foreign_key_list'] for t in tables.values())
assert [r['name'] for r in tables['local_thread_catalog']['table_info']][-2:]==['trial_conversation_type','chatgpt_async_status']
schema={'evidence_class':'all_exact_installed_source_migrations_applied_to_disposable_in_memory_sqlite_NOT_runtime_schema','sqlite_version':sqlite3.sqlite_version,'table_count':len(tables),'explicit_index_count':len([r for r in master if r['type']=='index' and r['sql']]),'trigger_count':0,'foreign_key_count':0,'semantic_canonical_sha256':sha(canonical(semantic)),'canonicalization':'UTF-8 JSON, sort_keys=true, ensure_ascii=false, separators comma/colon; digest excludes trailing LF; excludes rootpage only; source-derived fresh database schema, not every historical migration path', 'semantic_projection':semantic,'disposable_sqlite_master_with_rootpages':master}
dump('complete-schema-projection.json',schema)
(OUT/'complete-schema.canonical.json').write_bytes(canonical(semantic)+b'\n')
(OUT/'complete-schema.sql').write_text('\n\n'.join(r['sql']+';' for r in sorted(master,key=lambda r:(r['type']!='table',r['name'])) if r['sql'])+'\nPRAGMA user_version = 34;\n',encoding='utf8')

catalog_bytes=read_asset(CATALOG_ASSET);catalog_source=catalog_bytes.decode('utf8')
windows=[]
for name,a,b in [('catalog-checkpoint-shape',446350,447050),('catalog-authoritative-removal',466561,467600),('catalog-tombstone-suppression',468100,468700)]:
 text=catalog_source[a:b];file=name+'.extracted.js';(OUT/file).write_text(text,encoding='utf8')
 windows.append({'claim':name,'asset':CATALOG_ASSET,'unicode_start':a,'unicode_end':b,'utf8_sha256':sha(text.encode()),'artifact':file})
# Include complete relevant class, making exact conditional and sequence/revision
# implementation inspectable rather than relying only on small hash windows.
class_start=catalog_source.index('class mU{db;hostId;hostKind;activeScan=null;')
class_end=catalog_source.index('function mV(',class_start)
(OUT/'catalog-class.extracted.js').write_text(catalog_source[class_start:class_end],encoding='utf8')
meta={'schema_version':1,'date':'2026-11-03','evidence_class':'static_exact_installed_source_derived_schema_only','runtime_schema_captured':False,'provider_started':False,'real_user_state_read':False,'production_mutation_admitted':False,'app_path':str(APP),'bundle_id':info['CFBundleIdentifier'],'desktop_version':info['CFBundleShortVersionString'],'desktop_build':info['CFBundleVersion'],'main_executable_sha256':sha((APP/'Contents/MacOS'/info['CFBundleExecutable']).read_bytes()),'app_asar_sha256':EXPECTED_ASAR,'migration_asset':ASSET,'migration_asset_sha256':sha(asset),'migration_declarations_unicode_start':start,'migration_declarations_unicode_end':end,'migration_declarations_utf8_sha256':sha(fragment.encode()),'migration_schema_unicode_start':schema_start,'migration_schema_unicode_end':schema_end,'migration_count':len(migrations),'catalog_asset':CATALOG_ASSET,'catalog_asset_sha256':sha(catalog_bytes),'catalog_class_unicode_start':class_start,'catalog_class_unicode_end':class_end,'catalog_class_utf8_sha256':sha(catalog_source[class_start:class_end].encode()),'source_windows':windows,'schema_semantic_canonical_sha256':sha(canonical(semantic)),'superseded_evidence':{'path':'compatibility/codex/complete-removal-research/desktop-catalog-schema-static.json','scope':'last two local_thread_catalog column order and incomplete whole-schema inventory only','reason':'Native migration manifest applies trial_conversation_type before chatgpt_async_status, despite declaration order. Prior file lists chatgpt_async_status first. Prior file unchanged.','correct_last_columns':['trial_conversation_type','chatgpt_async_status']},'runtime_admission_requirements':['validate actual complete schema and migration predecessors/checksums under locked exact physical/root identity','reject unrecognized tables/indexes/triggers/FKs or migration records','source-derived fresh schema does not independently prove every historical schema route','exact-profile actual two-cycle isolated reconstruction acceptance remains required']}
dump('provenance.json',meta)
print(json.dumps({'out':str(OUT),'migration_count':len(migrations),'table_count':len(tables),'schema_semantic_sha256':meta['schema_semantic_canonical_sha256'],'migration_declarations_sha256':meta['migration_declarations_utf8_sha256'],'catalog_asset_sha256':meta['catalog_asset_sha256'],'catalog_class_start':class_start,'catalog_class_end':class_end},indent=2))
