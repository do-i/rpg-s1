#!/usr/bin/env python3
"""Snapshot map-art usage for the Rusted Kingdoms style audit (requires ImageMagick)."""
import argparse
import csv
import hashlib
import json
import subprocess
import xml.etree.ElementTree as ET
from collections import Counter, defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCENARIO = REPO / 'assets/scenarios/rusted_kingdoms'
TILESETS = SCENARIO / 'media/tilesets'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=REPO / 'docs/tile-style-audit')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    classifications = json.loads(Path(__file__).with_name('families.json').read_text())
    classified = {}
    for family in classifications:
        for atlas, ids in family['scope'].items():
            for tile_id in ids:
                key = (atlas, tile_id)
                if key in classified:
                    raise ValueError(f'Overlapping family assignment: {key}')
                classified[key] = family
    counts = Counter()
    maps = defaultdict(set)
    per_map = defaultdict(Counter)
    source_paths = set(TILESETS.glob('*.tsx'))
    inputs = set()
    playable = []
    excluded = []
    # Game map IDs use matching YAML and TMX basenames. Editor examples are not gameplay.
    for path in sorted((SCENARIO / 'media/maps').glob('*.tmx')):
        metadata = SCENARIO / 'data/maps' / (path.stem + '.yaml')
        if not metadata.exists() or 'template' in path.stem:
            excluded.append(path.relative_to(REPO).as_posix())
            continue
        playable.append(path.relative_to(REPO).as_posix())
        inputs.update((path, metadata))
        root = ET.parse(path).getroot()
        refs = []
        for node in root.findall('tileset'):
            source = node.get('source')
            if source is None:
                raise ValueError(f'Inline tileset needs explicit support: {path}')
            tsx = (path.parent / source).resolve()
            source_paths.add(tsx)
            refs.append((int(node.attrib['firstgid']), tsx))
        refs.sort()
        tilecounts = {source: int(ET.parse(source).getroot().attrib['tilecount']) for _, source in refs}
        def add(value):
            gid = value & 0x0fffffff  # Strip all Tiled transform flags.
            if not gid:
                return
            first, source = next((f, s) for f, s in reversed(refs) if gid >= f)
            tile_id = gid - first
            if not 0 <= tile_id < tilecounts[source]:
                raise ValueError(f'Invalid GID {gid} in {path}')
            key = (source.stem, tile_id)
            counts[key] += 1
            maps[key].add(path.stem)
            per_map[source.stem][path.stem] += 1
        def visit(node, visible=True):
            visible = visible and node.get('visible', '1') != '0' and float(node.get('opacity', '1')) > 0
            if not visible:
                return
            if node.tag == 'layer':
                data = node.find('data')
                if data is None or data.get('encoding') != 'csv' or data.find('chunk') is not None:
                    raise ValueError(f'Unsupported layer encoding in {path}')
                values = [int(v) for v in (data.text or '').split(',') if v.strip()]
                if len(values) != int(node.attrib['width']) * int(node.attrib['height']):
                    raise ValueError(f'Wrong layer cell count in {path}')
                for value in values:
                    add(value)
            if node.tag == 'object' and node.get('gid') is not None:
                add(int(node.attrib['gid']))
            for child in node:
                visit(child, visible)
        visit(root)
    rows = []
    for path in sorted(source_paths):
        inputs.add(path)
        root = ET.parse(path).getroot()
        image = root.find('image')
        if image is None:
            raise ValueError(f'Image collection needs explicit support: {path}')
        png = path.parent / image.attrib['source']
        inputs.add(png)
        width, height = int(image.attrib['width']), int(image.attrib['height'])
        tw, th = int(root.attrib['tilewidth']), int(root.attrib['tileheight'])
        columns = int(root.attrib['columns'])
        raw = subprocess.check_output(['magick', str(png), '-depth', '8', 'rgba:-'])
        if len(raw) != width * height * 4:
            raise ValueError(f'PNG dimensions differ from TSX metadata: {png}')
        properties = {int(t.attrib['id']): {p.attrib['name']: p.get('value', p.text or '')
                      for p in t.findall('properties/property')} for t in root.findall('tile')}
        for tile_id in range(int(root.attrib['tilecount'])):
            x0, y0 = tile_id % columns * tw, tile_id // columns * th
            nonempty = any(raw[(y * width + x) * 4 + 3]
                           for y in range(y0, min(y0 + th, height))
                           for x in range(x0, min(x0 + tw, width)))
            family = classified.get((path.stem, tile_id))
            if path.stem == 'terrain-v7':
                disposition, family_id = 'excluded', 'terrain-v7'
            elif path.stem == 'collision':
                disposition, family_id = 'technical', 'collision'
            elif not nonempty:
                disposition, family_id = 'transparent-slot', ''
            elif family:
                disposition, family_id = family['state'], family['key']
            else:
                raise ValueError(f'Nonempty tile lacks an explicit style disposition: {path.stem}:{tile_id}')
            key = (path.stem, tile_id)
            rows.append(dict(tileset=path.stem, local_id=tile_id,
                description=properties.get(tile_id, {}).get('desc', ''),
                has_pixels=int(nonempty), placements=counts[key], map_count=len(maps[key]),
                disposition=disposition, family=family_id, maps=';'.join(sorted(maps[key]))))
    # Standalone variants are not referenced by a TSX/map; keep them in the inventory.
    for name in ('stone_stairs_up_right_32x32', 'wood_stairs_up_right_32x32_v4'):
        path = TILESETS / (name + '.png')
        inputs.add(path)
        if not path.exists():
            raise ValueError(f'Missing standalone variant: {path}')
        rows.append(dict(tileset=name, local_id=0, description='Standalone 32x32 stair PNG',
            has_pixels=1, placements=0, map_count=0, disposition='redesign', family='stairs', maps=''))
    # Stamps are editor recipes, not extra player exposure; hash them for migration review.
    inputs.update(TILESETS.glob('stamps/*.stamp'))
    inputs.update((SCENARIO / 'media/map_templates').glob('*.tmx'))
    if (TILESETS / 'sample.tmx').exists():
        inputs.add(TILESETS / 'sample.tmx')
    inputs.update((Path(__file__), Path(__file__).with_name('families.json')))
    with (args.output / 'usage.csv').open('w') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    snapshot = dict(playable_maps=playable, excluded_maps=excluded,
        editor_sample='assets/scenarios/rusted_kingdoms/media/tilesets/sample.tmx (outside gameplay map directory)',
        map_counts={name: dict(sorted(values.items())) for name, values in sorted(per_map.items())},
        sources={p.relative_to(REPO).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                 for p in sorted(inputs)})
    (args.output / 'sources.json').write_text(json.dumps(snapshot, indent=2) + '\n')
    print(f'{len(playable)} playable maps; {len(rows)} atlas cells/standalone assets; {len(inputs)} hashed inputs')
    for family in classifications:
        selected = [r for r in rows if r['family'] == family['key']]
        map_ids = {m for r in selected for m in r['maps'].split(';') if m}
        placements = sum(r['placements'] for r in selected)
        print(f"{family['key']}: {placements} placements / {len(map_ids)} maps")


if __name__ == '__main__':
    main()
