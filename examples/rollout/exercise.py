"""Local required-evidence consumption demo; no identity, grant, or hosted gate."""
import asyncio, copy, hashlib, json, pathlib, subprocess, sys
from datetime import timedelta
from importlib.metadata import version
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client


def digest(data):
    return 'sha256:' + hashlib.sha256(data).hexdigest()


def snapshot(root):
    return {str(p.relative_to(root)): digest(p.read_bytes())
            for p in root.rglob('*') if p.is_file()}


async def main():
    assert version('mcp') == '1.29.0'
    cli, mcp, root, config_path = sys.argv[1:]
    root, config_path = pathlib.Path(root), pathlib.Path(config_path)
    config_bytes = config_path.read_bytes()
    config = json.loads(config_bytes)
    source_before = snapshot(root)
    # The fixed controller requires BOTH adapters throughout this exercise.
    # Disabling one cannot shrink requirements or convert old history to new evidence.
    required = frozenset(('cli', 'mcp'))
    enabled = set(required)
    history, bundles, current = [], [], {}
    counter = 0
    explicit_opt_in = False
    shadow_matches = False

    def retain(adapter, bundle):
        raw = json.dumps(bundle, sort_keys=True, separators=(',', ':')).encode()
        history.append(raw)
        bundles.append(bundle)
        current[adapter] = bundle
        return bundle

    def cli_check(cancel=False):
        nonlocal counter
        assert 'cli' in enabled
        counter += 1
        request = {k: copy.deepcopy(v) for k, v in config.items() if k != 'requestKey'}
        request['apiVersion'] = 'specguard.cli-check/v1alpha1'
        request['invocation']['run_id'] = f'rollout-cli-{counter}'
        path = config_path.parent / f'request-{counter}.json'
        path.write_text(json.dumps(request))
        args = [cli, 'check', str(root), str(path)] + (['--cancel'] if cancel else [])
        output = subprocess.run(args, capture_output=True, check=False)
        assert output.returncode == (4 if cancel else 0), output.stderr.decode()
        return retain('cli', json.loads(output.stdout))

    def signature(bundle):
        e = bundle['envelope']
        return {k: e[k] for k in ('producer', 'binding', 'coverage', 'decision', 'runStatus')} | {
            'contractDigest': (e['artifacts']['contract'] or {}).get('digest'),
        }

    def required_consume():
        return (explicit_opt_in and shadow_matches and required <= enabled
                and all(a in current and current[a]['envelope']['runStatus'] == 'completed'
                        and current[a]['envelope']['decision'] == 'ALLOW' for a in required)
                and signature(current['cli']) == signature(current['mcp']))

    # Advisory really executes the original CLI but cannot enable required consumption.
    advisory = cli_check()
    assert not required_consume()
    params = StdioServerParameters(command=mcp, args=['--local-readonly-v1', str(root), str(config_path), digest(json.dumps(list(config_bytes), separators=(',', ':')).encode())])
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write, read_timeout_seconds=timedelta(seconds=15)) as session:
            await session.initialize()

            async def mcp_check():
                assert 'mcp' in enabled
                result = await session.call_tool('specguard_check', {'requestKey': config['requestKey']})
                assert not result.isError
                envelope = result.structuredContent['envelope']
                bundle = {'envelope': envelope}
                for key in ('contract', 'facts', 'report', 'domain'):
                    reference = envelope['artifacts'][key]
                    if key == 'domain':
                        reference = reference[0]
                    evidence = await session.call_tool('specguard_evidence', {'requestKey': config['requestKey'], 'uri': reference['uri']})
                    assert not evidence.isError
                    data = evidence.structuredContent['text'].encode()
                    assert digest(data) == reference['digest']
                    bundle[key] = list(data)
                return retain('mcp', bundle)

            shadow = await mcp_check()
            # Actual cancellation produces the discrepant run; no envelope is edited.
            cli_check(cancel=True)
            shadow_matches = signature(current['cli']) == signature(shadow)
            explicit_opt_in = True
            unresolved_blocked = not required_consume()
            assert unresolved_blocked
            # Resolve by obtaining a NEW successful CLI run and comparing exact bindings.
            cli_check()
            shadow_matches = signature(current['cli']) == signature(current['mcp'])
            assert shadow_matches
            explicit_opt_in = False
            opt_in_required = not required_consume()
            explicit_opt_in = True
            before_rollback = required_consume()
            assert before_rollback
            prior = tuple(history)
            prior_current = copy.deepcopy(current)
            enabled.remove('cli')
            cli_disabled = not required_consume()
            await mcp_check()  # Independent MCP adapter still performs a real fresh read.
            other_reads = current['mcp']['envelope']['runId'] != prior_current['mcp']['envelope']['runId']
            assert current['cli'] == prior_current['cli']
            enabled.add('cli')
            enabled.remove('mcp')
            mcp_disabled = not required_consume()
            cli_check()  # Independent CLI still works; missing MCP remains unavailable.
            assert not required_consume()
            # Old MCP evidence can still be queried on the retained session for audit only.
            ref = prior_current['mcp']['envelope']['artifacts']['report']
            audit = await session.call_tool('specguard_evidence', {'requestKey': config['requestKey'], 'uri': ref['uri']})
            assert digest(audit.structuredContent['text'].encode()) == ref['digest']
            preserved = tuple(history[:len(prior)]) == prior
    assert source_before == snapshot(root)
    assert config_path.read_bytes() == config_bytes
    print(json.dumps({'profile':'local-unverified-controller-demo', 'advisoryWrites':False,
        'unresolvedDifferenceBlocksEnable':unresolved_blocked, 'explicitOptInRequired':opt_in_required,
        'requiredConsumptionBeforeRollback':before_rollback, 'cliDisabledRejectsRequired':cli_disabled,
        'mcpDisabledRejectsRequired':mcp_disabled, 'otherAdapterStillReads':other_reads,
        'historyPreserved':preserved, 'sourcePreserved':True, 'executionAuthorized':False,
        'newRuns':len(history), 'historyDigests':[digest(b) for b in history], 'bundles':bundles}))

asyncio.run(main())
