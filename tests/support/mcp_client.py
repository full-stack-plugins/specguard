"""Actual pinned SDK client; no hosted UI or production identity claim."""
import asyncio, hashlib, pathlib, sys
from datetime import timedelta
from importlib.metadata import version
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

def snapshot(root):
    return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in root.rglob('*') if p.is_file()}

async def main():
    assert version('mcp') == '1.29.0'
    binary, root, config, digest = sys.argv[1:]
    root = pathlib.Path(root)
    before = snapshot(root)
    params = StdioServerParameters(command=binary, args=['--local-readonly-v1', str(root), config, digest])
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write, read_timeout_seconds=timedelta(seconds=15)) as session:
            init = await session.initialize()
            assert init.protocolVersion == '2025-11-25'
            tools = await session.list_tools()
            assert len(tools.tools) == 3 and all(t.annotations.readOnlyHint for t in tools.tools)
            denied = await session.call_tool('specguard_check', {'requestKey':'wrong'})
            assert denied.isError
            result = await session.call_tool('specguard_check', {'requestKey':'fixture-controller-key'})
            assert result.structuredContent['envelope']['decision'] == 'ALLOW'
            uri = result.structuredContent['envelope']['artifacts']['report']['uri']
            evidence = await session.call_tool('specguard_evidence', {'requestKey':'fixture-controller-key', 'uri':uri})
            assert not evidence.isError
            assert evidence.structuredContent['executionAuthorized'] is False
    assert before == snapshot(root)
    print('mcp-python/1.29.0: initialized/list/denied/check/evidence; source unchanged')
asyncio.run(main())
