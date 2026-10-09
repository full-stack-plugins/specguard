"""Generate documented draft-2020-12 shapes. Rust performs semantic validation."""
import json
from pathlib import Path
out = Path(__file__).resolve().parents[1] / 'schemas/specguard-domain'
S = {'type': 'string'}
N = {'type': 'integer', 'minimum': 0}
B = {'type': 'boolean'}
V = {'const': 'specguard.domain/v1alpha1'}
def obj(**fields):
    return {'type':'object','additionalProperties':False,'required':list(fields),'properties':fields}
def arr(item): return {'type':'array','items':item}
def ref(name): return {'$ref':f'#/$defs/{name}'}
def enum(*values): return {'enum':list(values)}
D = {}
D['Identity'] = obj(namespace=S, id=S)
D['SourceRef'] = obj(path=S,line=N)
D['SourceStatus'] = obj(path=S,status=enum('complete','unsupported','malformed','limit','io_error','conflict'),reason=S)
D['Requirement'] = obj(key=ref('Identity'),text=S,source=ref('SourceRef'))
D['Acceptance'] = obj(key=ref('Identity'),requirement=ref('Identity'),text=S,source=ref('SourceRef'))
D['TraceEdge'] = obj(**{'from':ref('Identity'),'to':ref('Identity'),'relation':enum('depends_on','traces_to_adr','traces_to_task'),'source':ref('SourceRef')})
D['ParseResult'] = obj(apiVersion=V,snapshotDigest=S,candidateOid=S,sources=arr(ref('SourceStatus')),requirements=arr(ref('Requirement')),acceptances=arr(ref('Acceptance')),edges=arr(ref('TraceEdge')))
D['SpecificationGraph'] = obj(parsed=ref('ParseResult'))
D['Limits'] = obj(maxFiles=N,maxBytes=N,maxLines=N,maxDepth=N,maxMillis=N)
D['SourceEntry'] = obj(path=S,format=S,namespace=S,digest=S)
D['SourceInventory'] = obj(apiVersion=V,entries=arr(ref('SourceEntry')),sources=arr(ref('SourceStatus')),limits=ref('Limits'))
D['CandidateBinding'] = obj(candidateOid=S,baseOid=S,objectFormat=S)
D['SourceSnapshot'] = obj(apiVersion=V,inventory=ref('SourceInventory'),binding=ref('CandidateBinding'),contents={'type':'object','additionalProperties':arr({'type':'integer','minimum':0,'maximum':255})},digest=S)
D['ApprovedBaseline'] = obj(apiVersion=V,repository=S,scope=arr(ref('Identity')),sourceRevision=S,sourceDigest=S,graphDigest=S,policyDigest=S,approvalRef=S,effectiveFrom={'type':'integer'},expiresAt={'type':'integer'},state=enum('proposed','approved','superseded','expired','revoked'),graph=ref('SpecificationGraph'))
D['TestObligation'] = obj(id=S,requirement=ref('Identity'),acceptance=ref('Identity'),source=ref('SourceRef'),textDigest=S,links=arr(ref('TraceEdge')))
D['ObligationSet'] = obj(apiVersion=V,kind={'const':'TestObligationSet'},baselineDigest=S,sourceDigest=S,candidateOid=S,scope=arr(ref('Identity')),sources=arr(ref('SourceStatus')),complete=B,authenticationProfile=S,obligations=arr(ref('TestObligation')))
for name, type_name in [('baseline','ApprovedBaseline'),('obligations','ObligationSet'),('parse-result','ParseResult'),('inventory','SourceInventory'),('snapshot','SourceSnapshot')]:
    value={'$schema':'https://json-schema.org/draft/2020-12/schema','$id':f'https://specguard.invalid/schema/domain/v1alpha1/{name}.json',**D[type_name],'$defs':D}
    (out/f'{name}.json').write_text(json.dumps(value,indent=2)+'\n')
# Independent native identity-registry schema; existing domain objects remain unchanged.
scenario = obj(title=S,id=S)
requirement = obj(title=S,id=S,scenarios=arr(scenario))
registry = obj(apiVersion={'const':'specguard.openspec-ids/v1'},documents=arr(obj(path=S,requirements=arr(requirement))))
(out/'openspec-identities.json').write_text(json.dumps({'$schema':'https://json-schema.org/draft/2020-12/schema',**registry},indent=2)+'\n')
