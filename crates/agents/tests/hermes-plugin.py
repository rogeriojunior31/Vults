"""Run the generated observer against Hermes callback shapes, without any model or provider."""
import importlib.util
import json
import os
import sys

spec = importlib.util.spec_from_file_location('vults_hermes_test', sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
records = []
def relay(command, **kwargs):
    assert command[1:3] == ['--agent', 'hermes']
    records.append((command[-1], json.loads(kwargs['input'])))
module.subprocess.run = relay
class Context:
    def __init__(self): self.hooks = {}; self.unloads = []
    def on_unload(self, callback): self.unloads.append(callback)
    def register_hook(self, event, callback): self.hooks[event] = callback
ctx = Context()
module.register(ctx)
assert len(ctx.hooks) == 10
args = {'path': '/w/a.py', 'old_string': 'before', 'new_string': 'after', 'password': 'must-not-relay'}
assert ctx.hooks['on_session_start'](session_id='s', user_message='must-not-relay') is None
assert ctx.hooks['pre_tool_call'](session_id='s', tool_name='patch', args=args, tool_call_id='call-1') is None
assert ctx.hooks['post_tool_call'](session_id='s', tool_name='patch', args=args, tool_call_id='call-1', result='must-not-relay') is None
ctx.hooks['post_tool_call'](session_id='s', tool_name='terminal', args={'command': 'false'}, status='error')
ctx.hooks['subagent_start'](parent_session_id='s', child_session_id='child', child_goal='must-not-relay')
ctx.hooks['subagent_stop'](parent_session_id='s', child_session_id='child')
ctx.hooks['on_session_end'](session_id='s', failed=False)
ctx.hooks['on_session_finalize'](session_id='s')
module._queue.join()
assert [e for e, _ in records] == ['SessionStart', 'UserPromptSubmit', 'PreToolUse', 'PostToolUse', 'PostToolUseFailure', 'SubagentStart', 'SubagentStop', 'Stop', 'SessionEnd']
assert all(p['session_id'] == 's' for _, p in records)
assert records[2][1]['tool_use_id'] == records[3][1]['tool_use_id'] == 'call-1'
assert records[2][1]['tool_input']['old_string'] == 'before'
assert args['password'] == 'must-not-relay'
assert 'must-not-relay' not in json.dumps(records)
os.environ['VULTS_RUN'] = '1'
ctx.hooks['on_session_start'](session_id='owned')
module._queue.join()
assert len(records) == 9, 'Owned runs cannot duplicate Flock sessions'

os.environ.pop('VULTS_RUN', None)
for _ in range(2):
    ctx.hooks['post_api_request'](session_id='s', api_request_id='api-1', api_duration=1.25,
        usage={'prompt_tokens': 420, 'total_tokens': 500, 'raw_usage':'must-not-relay'}, context_length=1000,
        response='must-not-relay', assistant_message='must-not-relay')
module._queue.join()
assert records[-1][0] == 'SessionMetrics'
assert records[-1][1]['elapsed_ms'] == 1250, 'A repeated API event must not double its duration'
assert records[-1][1]['context_tokens'] == 420
assert records[-1][1]['context_limit'] == 1000
assert 'must-not-relay' not in json.dumps(records)

boundary = len(records)
ctx.hooks['on_session_finalize'](session_id='desktop', platform='desktop')
ctx.hooks['on_session_reset'](session_id='new', old_session_id='old', new_session_id='new')
ctx.hooks['on_session_reset'](session_id='desktop', platform='desktop')
module._queue.join()
assert [(event, payload['session_id']) for event,payload in records[boundary:]] == [
    ('Stop','desktop'), ('SessionEnd','old'), ('SessionStart','new'), ('SessionStart','desktop')]
ctx.hooks['on_session_delete'](session_id='desktop')
module._queue.join()
assert records[-1][0] == 'SessionEnd' and records[-1][1]['session_id'] == 'desktop'

for cleanup in ctx.unloads: cleanup()
replacement = Context()
module.register(replacement)
assert len(replacement.hooks) == 10, 'Reload must register the hooks again even if the module is cached'
for cleanup in replacement.unloads: cleanup()
