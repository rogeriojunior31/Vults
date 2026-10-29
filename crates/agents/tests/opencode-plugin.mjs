// Exercise the generated production plugin against both hook APIs, without an agent or model.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
const plugin = (await import(pathToFileURL(process.env.VULTS_PLUGIN_MODULE))).default;
const args = { filePath: '/w/a.ts', oldString: 'before', newString: 'after' };
async function untilRecords(count) {
  let records;
  for (let attempt = 0; attempt < 100; attempt++) {
    try { records = readFileSync(process.env.VULTS_PLUGIN_EVENTS, 'utf8').trim().split('\n').map(JSON.parse); } catch {}
    if (records?.length === count) return records;
    await new Promise(resolve => setTimeout(resolve, 20));
  }
  assert.equal(records?.length, count);
}
const v1 = await plugin.server({ client: {}, directory: '/w' });
await v1['tool.execute.before']({ sessionID: 'v1', callID: 'call1', tool: 'edit' }, { args });
await v1['tool.execute.after']({ sessionID: 'v1', callID: 'call1', tool: 'edit' });
await untilRecords(2);
const callbacks = {};
const cleanup = await plugin.setup({
  location: { directory: '/w' },
  event: { async *subscribe() {} },
  session: { async hook() {} },
  tool: { async hook(name, callback) { callbacks[name] = callback; } },
});
callbacks['execute.before']({ sessionID: 'v2', id: 'call2', tool: 'edit', input: args });
callbacks['execute.after']({ sessionID: 'v2', id: 'call2', tool: 'edit', status: 'success' });
const records = await untilRecords(4);
for (const session of ['v1', 'v2']) {
  const events = records.filter(event => event.session_id === session);
  assert.deepEqual(events.map(event => event.hook_event_name), ['PreToolUse', 'PostToolUse']);
  assert.deepEqual(events.map(event => event.tool_use_id), session === 'v1' ? ['call1', 'call1'] : ['call2', 'call2']);
  assert.deepEqual(events[1].tool_input, { file_path: '/w/a.ts', old_string: 'before', new_string: 'after' });
}
cleanup();

// Exercise failure, parallel same-file calls, permission routing and duplicate lifecycle events.
const pending = [];
let wake;
const replies = [];
const callbacks2 = {};
const abortableFeed = async function* ({ signal }) {
  signal.addEventListener('abort', () => wake?.(), { once: true });
  while (!signal.aborted) {
    if (!pending.length) await new Promise(resolve => { wake = resolve; });
    if (signal.aborted) return;
    while (pending.length) yield pending.shift();
  }
};
const emit = event => { pending.push(event); wake?.(); };
const cleanup2 = await plugin.setup({
  location: { directory: '/w' },
  event: { subscribe: abortableFeed },
  session: { async hook() {} },
  tool: { async hook(name, callback) { callbacks2[name] = callback; } },
  permission: { async reply(reply) { replies.push(reply); } },
});
callbacks2['execute.before']({ sessionID: 'parallel', id: 'a', tool: 'edit', input: args });
callbacks2['execute.before']({ sessionID: 'parallel', id: 'b', tool: 'edit', input: { ...args, oldString: 'second', newString: 'replacement', directory: '/w/child' } });
emit({ id: 'permission-a', type: 'permission.asked', data: { id: 'request-a', sessionID: 'parallel', source: { id: 'a' } } });
await untilRecords(7);
for (let attempt = 0; !replies.length && attempt < 100; attempt++) await new Promise(resolve => setTimeout(resolve, 10));
assert.deepEqual(replies, [{ sessionID: 'parallel', requestID: 'request-a', decision: 'once' }]);
callbacks2['execute.after']({ sessionID: 'parallel', id: 'b', tool: 'edit', status: 'success' });
callbacks2['execute.after']({ sessionID: 'parallel', id: 'a', tool: 'edit', status: 'error', error: { message: 'edit failed' } });
emit({ id: 'done-event', type: 'session.execution.succeeded', data: { sessionID: 'parallel' } });
emit({ id: 'done-event', type: 'session.execution.succeeded', data: { sessionID: 'parallel' } });
emit({ id: 'superseded-event', type: 'session.execution.interrupted', data: { sessionID: 'parallel', reason: 'superseded' } });
emit({ id: 'interrupted-event', type: 'session.execution.interrupted', data: { sessionID: 'parallel', reason: 'cancelled' } });
const all = await untilRecords(11);
const parallel = all.filter(event => event.session_id === 'parallel');
const done = parallel.find(event => event.hook_event_name === 'PostToolUse');
assert.equal(done.tool_use_id, 'b');
assert.equal(done.tool_input.old_string, 'second');
assert.equal(done.tool_input.workdir, '/w/child');
const failed = parallel.find(event => event.hook_event_name === 'PostToolUseFailure');
assert.equal(failed.tool_use_id, 'a');
assert.equal(failed.tool_input.old_string, 'before');
assert.equal(failed.error, 'edit failed');
const permission = parallel.find(event => event.hook_event_name === 'PermissionRequest');
assert.equal(permission.tool_use_id, 'a');
assert.equal(permission.tool_input.old_string, 'before');
assert.equal(parallel.filter(event => event.hook_event_name === 'Stop').length, 2);
cleanup2();
