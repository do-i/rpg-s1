const assert = require('node:assert/strict');
const { test, after } = require('node:test');
const { mkdtempSync, rmSync, writeFileSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { resolve, join } = require('node:path');
const { spawnSync } = require('node:child_process');

// Test the actual TypeScript mutation modules using the existing compiler;
// all generated JavaScript stays in a disposable directory.
const output = mkdtempSync(join(tmpdir(), 'rpg-editor-web-test-'));
after(() => rmSync(output, { recursive: true, force: true }));
writeFileSync(join(output, 'package.json'), '{"type":"commonjs"}');
const compilerEnv = { ...process.env };
delete compilerEnv.NODE_TEST_CONTEXT;
const compiled = spawnSync(process.execPath, [
  resolve('node_modules/typescript/bin/tsc'), 'src/api.ts', 'src/ops.ts',
  '--outDir', output, '--module', 'commonjs', '--target', 'ES2022', '--strict', '--skipLibCheck',
], { encoding: 'utf8', env: compilerEnv });
assert.equal(compiled.status, 0, compiled.stdout + compiled.stderr);
const api = require(join(output, 'api.js'));
const { applyOp, OpStack, PartialMutationError, recoverMutation } = require(join(output, 'ops.js'));

const request = { source_map: 'source', source_rect_px: [32, 32, 32, 32], target_map: 'target', target_tile: [1, 1] };
function edge(id = 2, revision = `rev${id}`, rect = request.source_rect_px) {
  return { id: `source#${id}`, source: 'source', target: 'target', portal_obj_id: id,
    source_tile: [1, 1], target_tile: [1, 1], source_rect_px: rect, revision };
}
function graph(revision = 'initial') {
  return { nodes: [{ map_id: 'source', revision }, { map_id: 'target', revision: 'target-initial' }], edges: [edge(1)] };
}
function response(body, status = 200, headers = {}) {
  return new Response(status === 204 ? null : JSON.stringify(body), { status, headers });
}
let calls, replies;
async function setup() {
  calls = [];
  replies = [response(graph())];
  global.fetch = async (url, options) => {
    calls.push({ url, options });
    const next = replies.shift();
    assert.ok(next, `unexpected request ${url}`);
    return next;
  };
  await api.fetchGraph();
  calls = [];
}
function createOp(two = false) {
  return { kind: 'create', requests: two ? [request, request] : [request], createdIds: [], label: 'create portal' };
}

for (const mode of ['undo', 'redo']) {
  test(`${mode}409 clears history and refreshes once without retrying the obsolete edit`, async () => {
    await setup();
    const history = new OpStack();
    const op = { kind: 'retarget', sourceMap: 'source', objId: 1,
      before: { target_map: 'target', target_tile: [1, 1], source_rect_px: null },
      after: { target_map: 'target', target_tile: [2, 1], source_rect_px: null }, label: 'move arrival' };
    history.push(op);
    if (mode === 'redo') {
      replies.push(response(edge(1, 'undone')));
      await history.undo();
      calls = [];
    }
    replies.push(response({ detail: 'Map source.tmx changed since this graph was loaded.' }, 409));
    let error;
    try { await history[mode](); } catch (caught) { error = caught; }
    assert.ok(error instanceof api.ApiError);
    assert.equal(error.status, 409);
    // Failed operations do not move the history cursor before recovery.
    assert.equal(history.canUndo, mode === 'undo');
    assert.equal(history.canRedo, mode === 'redo');
    let refreshes = 0, drafts = 0;
    replies.push(response(graph('external')));
    const message = await recoverMutation(error, history, async () => { refreshes++; await api.fetchGraph(); }, () => { drafts++; });
    assert.match(message, /Conflict:.*Graph refreshed; undo\/redo history cleared/);
    assert.equal(refreshes, 1);
    assert.equal(drafts, 1);
    assert.equal(history.canUndo, false);
    assert.equal(history.canRedo, false);
    assert.equal(calls.filter(call => call.options?.method === 'PATCH').length, 1);
    assert.equal(calls.filter(call => call.url === '/api/graph').length, 1);
    assert.equal(await history[mode](), null);
    assert.equal(calls.length, 2);
  });
}

test('creates and inverse operations advance revisions and track new IDs on redo', async () => {
  await setup();
  const history = new OpStack(), op = createOp();
  replies.push(response(edge(2, 'created'), 201));
  await applyOp(op); history.push(op);
  assert.equal(calls[0].options.headers['If-Match'], 'initial');
  replies.push(response(null, 204, { 'X-Map-Revision': 'deleted', 'X-Restore-Token': 'unused-create-receipt' }));
  assert.equal(await history.undo(), op.label);
  assert.equal(calls[1].options.headers['If-Match'], 'created');
  replies.push(response(edge(3, 'recreated'), 201));
  await history.redo();
  assert.deepEqual(op.createdIds, [{ sourceMap: 'source', objId: 3 }]);
  assert.equal(calls[2].options.headers['If-Match'], 'deleted');
  replies.push(response(null, 204, { 'X-Map-Revision': 'deleted-again', 'X-Restore-Token': 'unused' }));
  await history.undo();
  assert.equal(calls[3].url, '/api/portals/source/3');
});

test('delete undo restores server-held fractional geometry and redo deletes its fresh ID', async () => {
  await setup();
  const history = new OpStack(), rect = [-3.5, 30.25, 9.75, 32.125];
  const op = { kind: 'delete', edge: edge(1, 'initial', rect), label: 'delete authored portal' };
  replies.push(response(null, 204, { 'X-Map-Revision': 'deleted', 'X-Restore-Token': 'receipt1' }));
  await applyOp(op); history.push(op);
  replies.push(response(edge(2, 'restored', rect), 201));
  await history.undo();
  assert.equal(calls[1].url, '/api/portals/restore');
  assert.deepEqual(JSON.parse(calls[1].options.body), { token: 'receipt1' });
  assert.deepEqual(op.edge.source_rect_px, rect);
  replies.push(response(null, 204, { 'X-Map-Revision': 'deleted2', 'X-Restore-Token': 'receipt2' }));
  await history.redo();
  assert.equal(calls[2].url, '/api/portals/source/2');
  assert.equal(op.restoreToken, 'receipt2');
});

test('partial reciprocal create explains saved portal and clears unreliable history', async () => {
  await setup();
  const history = new OpStack(), op = createOp(true);
  replies.push(response(edge(2, 'created'), 201), response({ detail: 'Arrival is blocked' }, 422));
  let error;
  try { await applyOp(op); } catch (caught) { error = caught; }
  assert.ok(error instanceof PartialMutationError);
  assert.deepEqual(op.createdIds, [{ sourceMap: 'source', objId: 2 }]);
  const message = await recoverMutation(error, history, async () => {}, () => {});
  assert.match(message, /Partial edit: 1 portal\(s\) saved/);
  assert.match(message, /history cleared/);
  assert.equal(calls.length, 2);
  assert.equal(history.canUndo, false);
});

for (const mode of ['undo', 'redo']) {
  test(`partial reciprocal ${mode} explains acknowledged changes and disables retry history`, async () => {
    await setup();
    const history = new OpStack(), op = createOp(true);
    replies.push(response(edge(2, 'a')), response(edge(3, 'b')));
    await applyOp(op); history.push(op);
    if (mode === 'redo') {
      replies.push(response(null, 204, { 'X-Map-Revision': 'c' }), response(null, 204, { 'X-Map-Revision': 'd' }));
      await history.undo();
    }
    calls = [];
    replies.push(mode === 'undo' ? response(null, 204, { 'X-Map-Revision': 'partial' }) : response(edge(4, 'partial')),
                 response({ detail: 'Map changed' }, 409));
    let error;
    try { await history[mode](); } catch (caught) { error = caught; }
    assert.ok(error instanceof PartialMutationError);
    const message = await recoverMutation(error, history, async () => {}, () => {});
    assert.match(message, mode === 'undo' ? /1 portal\(s\) deleted/ : /1 portal\(s\) saved/);
    assert.equal(history.canUndo, false);
    assert.equal(history.canRedo, false);
    assert.equal(calls.length, 2);
    assert.equal(await history[mode](), null);
  });
}

test('graph refresh detects external revisions even after an acknowledged edit', async () => {
  await setup();
  replies.push(response(edge(2, 'acknowledged')));
  await api.createPortal(request);
  replies.push(response(graph('acknowledged')));
  assert.equal((await api.fetchGraph()).revisionsChanged, false);
  replies.push(response(graph('external-after-publication')));
  assert.equal((await api.fetchGraph()).revisionsChanged, true);
});

test('ordinary validation error is readable and leaves prior history usable', async () => {
  await setup();
  const history = new OpStack(), op = createOp();
  replies.push(response(edge(2, 'created'))); await applyOp(op); history.push(op);
  replies.push(response({ detail: 'target_tile: arrival blocked' }, 422));
  let error;
  try { await api.createPortal(request); } catch (caught) { error = caught; }
  let refreshes = 0;
  const message = await recoverMutation(error, history, async () => { refreshes++; }, () => { assert.fail('ordinary error discarded drafts'); });
  assert.equal(message, 'Error: target_tile: arrival blocked');
  assert.equal(refreshes, 1);
  assert.equal(history.canUndo, true);
});


test('expired restoration receipt clears deletion undo history and never falls back to lossy create', async () => {
  await setup();
  const history = new OpStack();
  const op = { kind: 'delete', edge: edge(1, 'initial', [-3.5, 30.25, 9.75, 32.125]), restoreToken: 'expired', label: 'delete authored portal' };
  history.push(op);
  replies.push(response({ detail: 'Portal restoration receipt is no longer available (expired).' }, 409));
  let error;
  try { await history.undo(); } catch (caught) { error = caught; }
  const message = await recoverMutation(error, history, async () => {}, () => {});
  assert.match(message, /expired/);
  assert.equal(history.canUndo, false);
  assert.equal(history.canRedo, false);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, '/api/portals/restore');
});
