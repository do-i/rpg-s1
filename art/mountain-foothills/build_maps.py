"""Rebuild the three authored Zone 05 layouts; metadata templates are the TMX files."""
from collections import deque
from pathlib import Path
import json
import math
import random
import subprocess
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
MEDIA = ROOT / 'assets/scenarios/rusted_kingdoms/media'
WORK = ROOT / 'temp/mountain-foothills'
PREFIX = 'zone_05_mountain_foothills_'
ATLAS_GID = 2388
HFLIP = 0x80000000
SPECS = {a['name']: a['atlas_rect'] for a in json.loads(
    (ROOT / 'art/mountain-foothills/prompts.json').read_text())['assets']}
WANG = {}
terrain = ET.parse(MEDIA / 'tilesets/terrain-v7.tsx').getroot()
for tile in terrain.find('wangsets/wangset').findall('wangtile'):
    values = tuple(map(int, tile.get('wangid').split(',')[1::2]))
    WANG.setdefault(values, int(tile.get('tileid'))+1)
raw = subprocess.check_output(['magick', str(MEDIA / 'tilesets/mountain_foothills.png'), '-depth', '8', 'RGBA:-'])
OPAQUE = {(x, y): any(raw[((y*32+dy)*864+x*32+dx)*4+3]
                     for dy in range(32) for dx in range(32))
          for y in range(17) for x in range(27)}

CONFIGS = [
    dict(segment='01', size=[76, 58], arrivals=[[12, 56], [74, 28]], npc=[19, 39],
         route=[(12, 58), (12, 45), (30, 45), (32, 40), (70, 40), (70, 28), (76, 28)],
         paths=[[(12, 45), (20, 40), (20, 38)],
                [(31, 40), (25, 28), (24, 18), (45, 18), (45, 22), (69, 22), (70, 28)],
                [(24, 18), (13, 18)], [(25, 18), (28, 9)],
                [(12, 45), (7, 39)], [(65, 40), (66, 49)]],
         bridges=[(38, 49, 40), (45, 56, 22)],
         props=[('ridge', -4, -3), ('ridge', 16, -4), ('ridge', 36, -4), ('ridge', 58, -3),
                ('ridge', -6, 53), ('ridge', 18, 53), ('ridge', 39, 53), ('ridge', 60, 53),
                ('ridge', 6, 7), ('ridge', 28, 18), ('ridge', 53, 8), ('ridge', 49, 41),
                ('shelter', 15, 31), ('cairn', 33, 44),
                ('pine', 3, 18), ('pine', 8, 19), ('pine', 15, 17),
                ('pine', 57, 18), ('pine', 63, 29), ('pine', 26, 46),
                ('pine', 54, 31), ('pine', 9, 30), ('pine', 33, 5),
                ('boulder', 26, 36), ('boulder', 6, 43), ('boulder', 65, 45),
                ('boulder', 61, 5), ('boulder', 3, 7)],
         chests=[('foothills_chest_01', 3, 65, 18), ('foothills_chest_02', 4, 32, 44),
                 ('foothills_chest_03', 5, 13, 18), ('foothills_chest_04', 6, 66, 49),
                 ('foothills_chest_05', 7, 7, 39), ('foothills_chest_06', 8, 28, 9)],
         spawns=[(10, 22), (22, 17), (26, 30), (31, 34), (34, 16), (35, 45),
                 (56, 39), (62, 34), (65, 23), (59, 18), (54, 28), (36, 7),
                 (24, 47), (9, 42), (63, 50), (70, 10)], board=[15, 48]),
    dict(segment='02', size=[80, 60], arrivals=[[1, 43], [78, 19]], npc=[60, 14],
         route=[(0, 43), (24, 43), (52, 43), (69, 43), (69, 19), (80, 19)],
         paths=[[(24, 43), (24, 19), (51, 20), (69, 19)],
                [(69, 19), (61, 15)], [(24, 19), (15, 22)],
                [(24, 19), (21, 8), (17, 8)], [(7, 43), (7, 48)],
                [(69, 43), (61, 38)], [(69, 43), (69, 54), (57, 53)]],
         bridges=[(28, 43, 43), (36, 51, 20)],
         props=[('ridge', -4, -3), ('ridge', 14, -3), ('ridge', 54, -3), ('ridge', 70, -3),
                ('ridge', -5, 55), ('ridge', 17, 55), ('ridge', 54, 55), ('ridge', 72, 55),
                ('ridge', 4, 9), ('ridge', 7, 30), ('ridge', 48, 27), ('ridge', 52, 44),
                ('shelter', 57, 5), ('cairn', 67, 9),
                ('pine', 3, 22), ('pine', 13, 43), ('pine', 26, 4),
                ('pine', 52, 12), ('pine', 72, 24), ('pine', 65, 32),
                ('pine', 4, 48), ('pine', 48, 49),
                ('boulder', 19, 22), ('boulder', 25, 48), ('boulder', 73, 7),
                ('boulder', 55, 21), ('boulder', 72, 50)],
         chests=[('foothills_02_chest_01', 3, 15, 22), ('foothills_02_chest_02', 4, 7, 48),
                 ('foothills_02_chest_03', 5, 17, 8), ('foothills_02_chest_04', 6, 61, 38),
                 ('foothills_02_chest_05', 7, 57, 53), ('foothills_02_chest_06', 8, 65, 16)],
         spawns=[(10, 38), (17, 28), (23, 48), (27, 34), (27, 14), (10, 20),
                 (20, 8), (50, 22), (57, 38), (66, 46), (71, 32), (52, 52),
                 (61, 55), (18, 52), (27, 25), (56, 18), (70, 9), (50, 40)], board=[22, 39]),
    dict(segment='03', size=[76, 64], arrivals=[[1, 47], [74, 15]], npc=[12, 18], boss=[58, 20],
         route=[(0, 47), (9, 47), (10, 35), (35, 35), (35, 23), (58, 23), (68, 15), (76, 15)],
         paths=[[(10, 35), (12, 21), (28, 16), (40, 17), (47, 24)],
                [(12, 21), (13, 18)], [(35, 35), (36, 53), (31, 52)],
                [(66, 24), (66, 29)], [(66, 29), (70, 44), (70, 56), (65, 56)],
                [(44, 17), (44, 7), (37, 7)]], bridges=[],
         props=[('ridge', -4, -3), ('ridge', 16, -4), ('ridge', 36, -4), ('ridge', 58, -3),
                ('ridge', -5, 59), ('ridge', 17, 59), ('ridge', 37, 59), ('ridge', 60, 59),
                ('ridge', 25, 6), ('ridge', 19, 25), ('ridge', 45, 34),
                ('ridge', 4, 52), ('ridge', 61, 45),
                ('shelter', 7, 5), ('cairn', 8, 13), ('workshop', 48, 7),
                ('pine', 3, 25), ('pine', 14, 34), ('pine', 28, 39),
                ('pine', 49, 43), ('pine', 65, 34), ('pine', 38, 42),
                ('pine', 18, 16), ('pine', 54, 26),
                ('boulder', 29, 19), ('boulder', 40, 29), ('boulder', 60, 54),
                ('boulder', 8, 38), ('boulder', 61, 5)],
         chests=[('foothills_03_chest_01', 4, 44, 17), ('foothills_03_chest_02', 5, 31, 52),
                 ('foothills_03_chest_03', 6, 66, 29), ('foothills_03_chest_04', 7, 10, 23),
                 ('foothills_03_chest_05', 8, 65, 56), ('foothills_03_chest_06', 9, 37, 7)],
         spawns=[(9, 35), (14, 30), (26, 35), (33, 40), (36, 25), (42, 22),
                 (44, 32), (49, 30), (63, 34), (70, 42), (49, 49), (39, 54),
                 (13, 50), (24, 20), (20, 16), (44, 9)], board=[14, 43]),
]


def distance(x, y, points):
    result = float('inf')
    for (ax, ay), (bx, by) in zip(points, points[1:]):
        dx, dy = bx-ax, by-ay
        t = max(0, min(1, ((x-ax)*dx+(y-ay)*dy)/(dx*dx+dy*dy)))
        result = min(result, math.hypot(x-ax-t*dx, y-ay-t*dy))
    return result


def water(segment, x, y):
    if segment == '01':
        return abs(x-(47+4*math.sin(y/9))) < 2.7
    if segment == '02':
        return abs(x-(38+6*math.sin(y/12))) < 5.6
    return ((x-23)/8)**2+((y-49)/7)**2 < 1


def neighbors(x, y):
    return [(x-1, y), (x+1, y), (x, y-1), (x, y+1)]


def build(config):
    segment = config['segment']
    W, H = config['size']
    rng = random.Random(50500+int(segment))
    p = MEDIA / f'maps/{PREFIX}{segment}.tmx'
    root = ET.parse(p).getroot()
    root.set('width', str(W))
    root.set('height', str(H))
    root.set('nextobjectid', str(max(c[1] for c in config['chests'])+1))
    if not any(ts.get('source') == '../tilesets/mountain_foothills.tsx' for ts in root.findall('tileset')):
        root.insert(len(root.findall('tileset')), ET.Element('tileset', firstgid=str(ATLAS_GID),
                                                            source='../tilesets/mountain_foothills.tsx'))
    refs = {Path(ts.get('source')).stem: int(ts.get('firstgid')) for ts in root.findall('tileset')}
    a = {l.get('name'): [0]*(W*H) for l in root.findall('layer')}
    depth, reasons, placements = {}, {}, []
    routes = [config['route'], *config['paths']]
    for y in range(H):
        for x in range(W):
            a['ground'][y*W+x] = rng.choice([1837, 1838, 1839])

    def inside(x, y):
        return 0 <= x < W and 0 <= y < H

    def put(layer, x, y, gid):
        if inside(x, y):
            a[layer][y*W+x] = gid

    def block(x, y, reason):
        if inside(x, y):
            reasons.setdefault((x, y), set()).add(reason)

    def paint(color, predicate):
        for y in range(H):
            for x in range(W):
                # Tiled corner order: top-right, bottom-right, bottom-left, top-left.
                key = tuple(color if predicate(xx, yy) else 0
                            for xx, yy in ((x+1, y), (x+1, y+1), (x, y+1), (x, y)))
                if any(key):
                    assert key in WANG, (color, key)
                    a['terrain'][y*W+x] = WANG[key]

    # Leave snow-free corners across every tile touched by water. Otherwise the
    # water pass erases a partial snow tile beside a full one, cutting the snow
    # off at a straight tile boundary instead of using its authored edge.
    wet_vertices = {(x, y) for y in range(H+1) for x in range(W+1) if water(segment, x, y)}
    shore_vertices = {(x+dx, y+dy) for x, y in wet_vertices
                      for dx in (-1, 0, 1) for dy in (-1, 0, 1)}

    def snow(x, y):
        near_path = min(distance(x, y, route) for route in routes) < 3.2
        field = math.sin(x*.19)+.6*math.cos(y*.17)+.45*math.cos((x+y)*.13)+y/H*.35
        return (x, y) not in shore_vertices and not near_path and field > (
            .3 if segment == '01' else -.05 if segment == '02' else -.3)

    paint(5, snow)
    paint(13 if segment == '01' else 15, lambda x, y: min(distance(x, y, route) for route in routes) < 2.0)
    paint(9 if segment == '02' else 8, lambda x, y: water(segment, x, y))
    for y in range(H):
        for x in range(W):
            if water(segment, x+.5, y+.5):
                block(x, y, 'meltwater')
    # Actual rope-bridge art spans both banks. Only the center deck row is walkable.
    for x0, x1, cy in config['bridges']:
        for y in range(cy-1, cy+2):
            for x in range(x0, x1+1):
                col = 0 if x == x0 else 2 if x == x1 else 1
                put('architecture', x, y, refs['floor']+45+(y-cy+1)*16+col)
                reasons.pop((x, y), None)
                if y != cy:
                    block(x, y, 'bridge rail')

    def place(name, x0, y0, flip=False):
        sx, sy, w, h = SPECS[name]
        occupied, cells = set(), []
        for y in range(h):
            for x in range(w):
                sc = sx+(w-1-x if flip else x)
                xx, yy = x0+x, y0+y
                if not inside(xx, yy) or not OPAQUE[sc, sy+y]:
                    continue
                layer = 'decoration' if y == h-1 else 'foreground'
                assert not a[layer][yy*W+xx], ('overlap', segment, name, xx, yy)
                a[layer][yy*W+xx] = ATLAS_GID+(sy+y)*27+sc+(HFLIP if flip else 0)
                if layer == 'foreground':
                    depth[xx, yy] = (h-y)*32
                occupied.add((x, y))
        if name == 'pine':
            footprint = [(1, h-1), (2, h-1)]
        else:
            # Ridges block only their bottom two rows. Their upper silhouette
            # stays in foreground so actors can walk behind the rock face.
            rows = 4 if name == 'shelter' else 2
            footprint = [(x, y) for y in range(max(0, h-rows), h) for x in range(w)]
        for x, y in footprint:
            if (x, y) in occupied:
                block(x0+x, y0+y, name)
                cells.append([x0+x, y0+y])
        placements.append(dict(object=name, position=[x0, y0], flip_horizontal=flip, collision=cells))

    for index, (name, x, y) in enumerate(config['props']):
        place(name, x, y, index % 3 == 2)
    # Perimeter trees are sparse; the map boundary itself follows the finite TMX bounds.
    for x in (-2, W-2):
        for y in (7, 18, 29, 40):
            if x == W-2 and segment == '03' and y == 7:
                continue
            if x == -2 and segment == '02' and y == 40:
                continue
            if x == W-2 and segment == '01' and y == 29:
                continue
            place('pine', x, y, x > 0)

    # Existing board identity remains map-scoped; its dialogue stays in the same file.
    bx, by = config['board']
    assert not a['decoration'][by*W+bx]
    put('decoration', bx, by, refs['outdoor']+12)
    block(bx, by, 'trail board')
    boxes = root.find("objectgroup[@name='item_boxes']")
    boxes[:] = []
    for name, id, x, y in config['chests']:
        assert not a['decoration'][y*W+x] and not a['foreground'][y*W+x], ('chest art overlap', segment, name)
        obj = ET.SubElement(boxes, 'object', id=str(id), name=name, gid=str(refs['item_box']),
                            x=str(x*32), y=str((y+1)*32), width='32', height='32')
        props = ET.SubElement(obj, 'properties')
        ET.SubElement(props, 'property', name='item_box_id', value=name)
        block(x, y, 'chest')
    for x, y in config['spawns']:
        assert not reasons.get((x, y)), ('blocked patrol', segment, x, y, reasons.get((x, y)))
        put('spawn_tile', x, y, refs['outdoor']+47)
    for (x, y), why in reasons.items():
        a['collision'][y*W+x] = refs['collision']
    for x0, x1, cy in config['bridges']:
        assert all(not a['collision'][cy*W+x] for x in range(x0, x1+1)), ('blocked bridge', segment, cy)
    if segment == '03':
        boss = root.find("objectgroup[@name='boss_enemy']/object")
        boss.set('x', str(config['boss'][0]*32))
        boss.set('y', str(config['boss'][1]*32))
    for obj in root.findall("objectgroup[@name='portals']/object"):
        if 'entry_' in obj.get('name'):
            geometry = (10*32, H*32-12, 5*32, 12) if segment == '01' else (0, (43 if segment == '02' else 47)*32-64, 12, 5*32)
            target = ('zone_04_ancient_ruins_03_sanctum', 31, 2) if segment == '01' else (
                PREFIX+('01' if segment == '02' else '02'), 74 if segment == '02' else 78, 28 if segment == '02' else 19)
        else:
            cy = 28 if segment == '01' else 19 if segment == '02' else 15
            geometry = (W*32-12, (cy-2)*32, 12, 5*32)
            target = (PREFIX+'02', 1, 43) if segment == '01' else (PREFIX+'03', 1, 47) if segment == '02' else ('town_04_frostholm', 19, 29)
        for key, value in zip(('x', 'y', 'width', 'height'), geometry):
            obj.set(key, str(value))
        props = {p.get('name'): p for p in obj.findall('properties/property')}
        for key, value in zip(('target_map', 'target_position_x', 'target_position_y'), target):
            props[key].set('value', str(value))
    for layer in root.findall('layer'):
        name = layer.get('name')
        layer.set('width', str(W))
        layer.set('height', str(H))
        if name in ('collision', 'spawn_tile'):
            layer.set('visible', '0')
        layer.find('data').text = '\n'+',\n'.join(','.join(map(str, a[name][y*W:(y+1)*W])) for y in range(H))+'\n'
        if name == 'foreground':
            props = layer.find('properties')
            if props is None:
                props = ET.Element('properties')
                layer.insert(0, props)
            props[:] = []
            ET.SubElement(props, 'property', name='sort_base_y_overrides', value=' '.join(
                f'{x},{y}:{value}' for (x, y), value in sorted(depth.items(), key=lambda v: (v[0][1], v[0][0]))))
    ET.indent(root, space=' ')
    p.write_text('<?xml version="1.0" encoding="UTF-8"?>\n'+ET.tostring(root, encoding='unicode').replace(' />', '/>')+'\n')
    seen = {tuple(config['arrivals'][0])}
    queue = deque(seen)
    while queue:
        for x, y in neighbors(*queue.popleft()):
            if inside(x, y) and not a['collision'][y*W+x] and (x, y) not in seen:
                seen.add((x, y))
                queue.append((x, y))
    for pos in [*config['arrivals'], *config['spawns'], config.get('boss', config['arrivals'][0])]:
        assert tuple(pos) in seen, ('unreachable', segment, pos)
    nx, ny = config['npc']
    for x, y in [(nx, ny+1), (nx+1, ny+1), (nx+1, ny+2)]:
        assert (x, y) in seen, ('NPC', segment, x, y)
    for name, _, x, y in config['chests']:
        assert any(v in seen for v in neighbors(x, y)), ('chest', segment, name)
    (WORK / (segment+'-collision.json')).write_text(json.dumps({f'{x},{y}': sorted(v) for (x, y), v in reasons.items()}, indent=2)+'\n')
    print(f'{segment}: {W}x{H}, {len(placements)} props, six reachable caches, {len(seen)} reachable cells')
    return dict(map=p.stem, size=[W, H], arrivals=config['arrivals'], npc_anchor=config['npc'],
                boss=config.get('boss'), placements=placements, chests=[dict(name=n, id=i, position=[x, y])
                for n, i, x, y in config['chests']], encounters=config['spawns'],
                reachable_cells=len(seen), bridges=config['bridges'], board=config['board'])


if __name__ == '__main__':
    WORK.mkdir(parents=True, exist_ok=True)
    reports = [build(config) for config in CONFIGS]
    (ROOT / 'art/mountain-foothills/layouts.json').write_text(json.dumps(dict(maps=reports), indent=2)+'\n')
