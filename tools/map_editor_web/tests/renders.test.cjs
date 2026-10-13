const assert = require('node:assert/strict');
const { test, after } = require('node:test');
const { mkdtempSync, rmSync, writeFileSync, symlinkSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { resolve, join } = require('node:path');
const { spawnSync } = require('node:child_process');

// Render the real image consumers without adding a DOM or browser dependency.
const output = mkdtempSync(join(tmpdir(), 'rpg-editor-render-test-'));
after(() => rmSync(output, { recursive: true, force: true }));
writeFileSync(join(output, 'package.json'), '{"type":"commonjs"}');
symlinkSync(resolve('node_modules'), join(output, 'node_modules'), 'dir');
const compilerEnv = { ...process.env };
delete compilerEnv.NODE_TEST_CONTEXT;
const compiled = spawnSync(process.execPath, [
  resolve('node_modules/typescript/bin/tsc'), 'src/api.ts', 'src/MapNode.tsx', 'src/TilePicker.tsx', 'src/SidePanel.tsx',
  '--outDir', output, '--module', 'commonjs', '--target', 'ES2022', '--strict', '--skipLibCheck', '--jsx', 'react-jsx',
], { encoding: 'utf8', env: compilerEnv });
assert.equal(compiled.status, 0, compiled.stdout + compiled.stderr);
const React = require('react');
const { renderToStaticMarkup } = require('react-dom/server');
const { ReactFlowProvider } = require('@xyflow/react');
const api = require(join(output, 'api.js'));
const { MapNode } = require(join(output, 'MapNode.js'));
const { TilePicker } = require(join(output, 'TilePicker.js'));
const { SidePanel } = require(join(output, 'SidePanel.js'));

function map(render_revision) {
  return { map_id: 'map with space', revision: 'unchanged-tmx', render_revision, display_name: 'Map',
    map_size_px: [128, 128], tile_size_px: [32, 32], npcs: [], item_boxes: [] };
}
function imageSources(info) {
  const node = renderToStaticMarkup(React.createElement(ReactFlowProvider, null, React.createElement(MapNode, {
    data: { mapId: info.map_id, renderRevision: info.render_revision, displayName: info.display_name,
      thumbWidth: 128, thumbHeight: 128, markers: [], arrivals: [], badges: [], onSelectEdge() {} },
  })));
  const picker = renderToStaticMarkup(React.createElement(TilePicker, {
    map: info, picked: null, onPick() {}, markers: [], maxWidth: 128, maxHeight: 128,
  }));
  const panel = renderToStaticMarkup(React.createElement(SidePanel, {
    selection: { kind: 'node', id: info.map_id }, nodesById: new Map([[info.map_id, info]]),
    edgesById: new Map(), onSelectMap() {}, onRetargetEdge() {}, onDeleteEdge() {}, width: 300,
  }));
  return [node, picker, panel].map(markup => markup.match(/<img[^>]+src="([^"]+)"/)[1]);
}

test('graph dependency refresh updates every preview URL while edit revision stays unchanged', async () => {
  let current = 'tsx-image-first';
  global.fetch = async (url, options) => {
    assert.equal(url, '/api/graph');
    assert.equal(options.cache, 'no-store');
    return new Response(JSON.stringify({ nodes: [map(current)], edges: [] }));
  };
  const before = await api.fetchGraph();
  const oldSources = imageSources(before.nodes[0]);
  current = 'tsx-image-next';
  const after = await api.fetchGraph();
  assert.equal(after.revisionsChanged, false, 'image edits should not discard portal undo history');
  const newSources = imageSources(after.nodes[0]);
  for (let index = 0; index < oldSources.length; index++) {
    assert.match(oldSources[index], /revision=tsx-image-first/);
    assert.match(newSources[index], /revision=tsx-image-next/);
    assert.notEqual(oldSources[index], newSources[index]);
  }
  assert.match(newSources[0], /map%20with%20space\/thumbnail/);
  assert.match(newSources[1], /map%20with%20space\/full/);
});

test('missing dependency and later repair replace the image URLs in every consumer', () => {
  const failed = imageSources(map(null));
  const repaired = imageSources(map('repaired-image'));
  for (let index = 0; index < failed.length; index++) {
    assert.match(failed[index], /revision=unavailable/);
    assert.match(repaired[index], /revision=repaired-image/);
    assert.notEqual(failed[index], repaired[index]);
  }
});
