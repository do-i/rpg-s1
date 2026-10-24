"""Rebuild only Zone 06 from authored layouts, retaining TMX IDs and identities."""
from collections import deque
from pathlib import Path
import json
import math
import random
import subprocess
import xml.etree.ElementTree as ET
from tile_transforms import compensate_flags

ROOT = Path(__file__).resolve().parents[2]
MEDIA = ROOT / 'assets/scenarios/rusted_kingdoms/media'
SOURCE = Path(__file__).resolve().parent
PREFIX = 'zone_06_mountain_pass_'
HFLIP = 0x80000000
WANG = {}
terrain = ET.parse(MEDIA / 'tilesets/terrain-v7.tsx').getroot()
colors = {c.get('name'): i for i,c in enumerate(terrain.findall('wangsets/wangset/wangcolor'),1)}
for wang in terrain.findall('wangsets/wangset'):
    local = {i: colors[c.get('name')] for i,c in enumerate(wang.findall('wangcolor'),1)}
    local[0] = 0
    for tile in wang.findall('wangtile'):
        key = tuple(local[v] for v in map(int,tile.get('wangid').split(',')[1::2]))
        WANG.setdefault(key,int(tile.get('tileid'))+1)
MP_LAYOUT = json.loads((SOURCE/'tile_layout.json').read_text())
MP_TILE_IDS = MP_LAYOUT['tile_ids']
SPECS = {}
for atlas, columns, folder in [('mountain_foothills', 27, 'mountain-foothills'), ('mountain_pass', 18, 'mountain-pass')]:
    raw = subprocess.check_output(['magick', str(MEDIA / f'tilesets/{atlas}.png'), '-depth', '8', 'RGBA:-'])
    for asset in json.loads((ROOT / f'art/{folder}/prompts.json').read_text())['assets']:
        sx, sy, w, h = asset['atlas_rect']
        if atlas == 'mountain_pass':
            opaque = {(x,y) for y in range(h) for x in range(w)
                      if str((sy+y)*columns+sx+x) in MP_TILE_IDS}
        else:
            opaque = {(x, y) for y in range(h) for x in range(w)
                      if any(raw[(((sy+y)*32+dy)*columns*32+(sx+x)*32+dx)*4+3]
                             for dy in range(32) for dx in range(32))}
        SPECS[asset['name']] = (atlas, columns, (sx, sy, w, h), opaque)

CONFIGS = [
    dict(segment='01', size=[76,58], arrivals=[[1,44],[74,16]], npc=[18,37], board=[12,41],
         route=[(0,44),(12,44),(23,40),(33,40),(39,29),(56,29),(64,16),(76,16)],
         paths=[[(12,44),(11,27),(23,22),(35,22),(39,29)],
                [(23,22),(25,13),(50,13),(56,29)],
                [(56,29),(64,42),(64,48)]],
         props=[('ridge',-4,-4),('ridge',17,-4),('ridge',39,-4),('ridge',61,-4),
                ('ridge',-5,54),('ridge',17,54),('ridge',39,54),('ridge',61,54),
                ('ridge',2,6),('ridge',32,15),('ridge',42,36),
                ('shelter',14,28),
                ('pine',2,22),('pine',4,37),('pine',25,42),('pine',34,47),
                ('pine',57,7),('pine',65,30),('pine',69,43),('pine',17,16),
                ('pine',30,5),('pine',3,46),('pine',68,3),
                ('boulder',28,31),('boulder',64,20),('boulder',59,49),('boulder',7,17)],
         chests=[('pass_chest_01',3,25,13),('pass_chest_02',4,64,48)],
         spawns=[(9,31),(23,24),(28,18),(36,12),(47,13),(51,25),(59,22),
                 (65,35),(61,43),(30,47),(40,32),(12,49),(44,47),(69,24)]),
    dict(segment='02', size=[84,64], arrivals=[[1,44],[82,18]], board=[25,27],
         route=[(0,44),(15,44),(24,38),(25,30),(30,27),(54,27),(58,30),(65,18),(84,18)],
         paths=[[(24,38),(25,49),(29,48),(54,48),(59,49),(66,41),(65,18)],
                [(25,30),(23,21),(16,14)],[(58,30),(57,13),(65,10)],
                [(59,49),(65,49)]],
         stone_bridge=[34,24], rope_bridge=[30,51,48], rope_break=[40,41],
         props=[('ridge',-4,-4),('ridge',17,-4),('ridge',55,-4),('ridge',74,-4),
                ('ridge',-5,59),('ridge',16,59),('ridge',55,59),('ridge',75,59),
                ('ridge',3,23),('ridge',3,48),('ridge',61,27),
                ('pine',3,9),('pine',8,10),('pine',23,5),('pine',26,12),
                ('pine',69,5),('pine',74,24),('pine',68,43),('pine',58,51),
                ('pine',19,47),('pine',3,33),('pine',75,49),
                ('boulder',12,15),('boulder',19,33),('boulder',65,52),
                ('boulder',68,37),('boulder',52,4)],
         chests=[('pass_02_chest_01',3,16,14),('pass_02_chest_02',4,65,49)],
         spawns=[(12,39),(22,36),(26,24),(21,16),(28,9),(24,53),(15,51),
                 (58,25),(62,16),(64,10),(70,21),(68,40),(62,46),(71,52),
                 (54,39),(76,43)]),
    dict(segment='03', size=[80,64], arrivals=[[1,46],[78,15]], board=[33,33], boss=[58,28],
         route=[(0,46),(16,46),(24,36),(37,36),(43,29),(58,29),(58,11),(69,11),(80,15)],
         paths=[[(16,46),(10,35),(8,25),(8,15),(28,15),(33,25),(43,29)],
                [(43,29),(47,43),(63,43),(63,51)]],
         props=[('ridge',-4,-4),('ridge',17,-4),('ridge',39,-4),('ridge',62,-4),
                ('ridge',-5,59),('ridge',17,59),('ridge',39,59),('ridge',61,59),
                ('ridge',36,14),('ridge',64,14),('gate',52,13),
                ('ridge',14,31),('ridge',28,46),
                ('pine',2,5),('pine',23,5),('pine',29,31),('pine',2,31),
                ('pine',6,47),('pine',49,46),('pine',69,34),('pine',69,49),
                ('pine',32,4),('pine',72,3),
                ('boulder',26,20),('boulder',38,38),('boulder',56,46),('boulder',11,18)],
         chests=[('pass_03_chest_01',4,11,15),('pass_03_chest_02',5,63,51)],
         spawns=[(10,41),(18,46),(27,42),(34,36),(35,27),(29,18),(9,28),
                 (8,12),(24,12),(44,40),(52,43),(64,45),(70,29),(69,8)]),
]


def distance(x, y, points):
    result = float('inf')
    for (ax,ay),(bx,by) in zip(points,points[1:]):
        dx,dy=bx-ax,by-ay
        t=max(0,min(1,((x-ax)*dx+(y-ay)*dy)/(dx*dx+dy*dy)))
        result=min(result,math.hypot(x-ax-t*dx,y-ay-t*dy))
    return result


def hazard(segment,x,y):
    if segment=='02':
        return abs(x-(42+3*math.sin(y/11)))<6.5
    if segment=='03':
        return ((x-19)/7.5)**2+((y-25)/5.5)**2<1
    return False


def build(config):
    segment=config['segment']; W,H=config['size']; rng=random.Random(60600+int(segment))
    p=MEDIA/f'maps/{PREFIX}{segment}.tmx'; root=ET.parse(p).getroot()
    root.set('width',str(W));root.set('height',str(H))
    refs={Path(t.get('source')).stem:int(t.get('firstgid')) for t in root.findall('tileset')}
    if segment=='01':
        for ts in root.findall('tileset'):
            if ts.get('source')=='../tilesets/mountain_pass.tsx':root.remove(ts)
    for atlas in ['mountain_foothills']+(['mountain_pass'] if segment!='01' else []):
        if atlas not in refs:
            end=max(int(t.get('firstgid'))+int(ET.parse((p.parent/t.get('source')).resolve()).getroot().get('tilecount')) for t in root.findall('tileset'))
            root.insert(len(root.findall('tileset')),ET.Element('tileset',firstgid=str(end),source=f'../tilesets/{atlas}.tsx'))
            refs[atlas]=end
    def atlas_gid(atlas,local,flip=False):
        tile=MP_TILE_IDS[str(local)] if atlas=='mountain_pass' else local
        flags=HFLIP if flip else 0
        if atlas=='mountain_pass':
            flags=compensate_flags(flags,MP_LAYOUT.get('tile_transforms',{}).get(str(local),0))
        return (refs[atlas]+tile)|flags
    a={l.get('name'):[0]*(W*H) for l in root.findall('layer')}
    reasons={};depth={};placements=[];routes=[config['route'],*config['paths']]
    def inside(x,y):return 0<=x<W and 0<=y<H
    def put(layer,x,y,gid):
        if inside(x,y):a[layer][y*W+x]=gid
    def block(x,y,why):
        if inside(x,y):reasons.setdefault((x,y),set()).add(why)
    def paint(color,predicate,outside=0):
        for y in range(H):
            for x in range(W):
                key=tuple(color if predicate(xx,yy) else outside for xx,yy in [(x+1,y),(x+1,y+1),(x,y+1),(x,y)])
                if color in key:a['terrain'][y*W+x]=WANG[key]
    for y in range(H):
        for x in range(W):a['ground'][y*W+x]=rng.choice([1837,1838,1839])
    # Keep full transition tiles around hazards; snow edges must not be cut by later paint.
    wet={(x,y) for y in range(H+1) for x in range(W+1) if hazard(segment,x,y)}
    shore={(x+dx,y+dy) for x,y in wet for dx in [-1,0,1] for dy in [-1,0,1]}
    def snow(x,y):
        field=math.sin(x*.16)+.7*math.cos(y*.19)+.35*math.cos((x-y)*.21)
        arena_margin = segment=='03' and ((x-58)/10.5)**2+((y-30)/8.5)**2<1
        return not arena_margin and (x,y) not in shore and min(distance(x,y,r) for r in routes)>3 and field>(-.45 if segment=='01' else -.8)
    paint(5,snow)
    paint(15,lambda x,y:min(distance(x,y,r) for r in routes)<2.0 or (segment=='03' and ((x-58)/9)**2+((y-30)/7)**2<1))
    paint(25 if segment=='02' else 8,lambda x,y:hazard(segment,x,y))
    if segment=='03':paint(11,lambda x,y:hazard(segment,19+(x-19)*1.3,25+(y-25)*1.3),outside=8)
    for y in range(H):
        for x in range(W):
            # Even partial black/ice tiles are blocked, avoiding an invisible walkable lip.
            if any(hazard(segment,xx,yy) for xx,yy in [(x,y),(x+1,y),(x,y+1),(x+1,y+1)]):block(x,y,'ravine' if segment=='02' else 'frozen tarn')
    if 'stone_bridge' in config:
        bx,by=config['stone_bridge']
        for y in range(by+1,by+6):
            for x in range(bx,bx+18):
                put('terrain',x,y,WANG[(15,15,15,15)])
        for x in [bx-2,bx-1,bx+18,bx+19]:
            for y in range(by+2,by+5):
                put('terrain',x,y,WANG[(15,15,15,15)])
    def place(name,x0,y0,flip=False):
        atlas,columns,(sx,sy,w,h),opaque=SPECS[name];cells=[]
        for y in range(h):
            for x in range(w):
                sc=w-1-x if flip else x;xx,yy=x0+x,y0+y
                if not inside(xx,yy) or (sc,y) not in opaque:continue
                # Gate lintel and complete upper silhouettes sort at each object's feet.
                layer='decoration' if y==h-1 else 'foreground'
                assert not a[layer][yy*W+xx],('art overlap',segment,name,xx,yy)
                put(layer,xx,yy,atlas_gid(atlas,(sy+y)*columns+sx+sc,flip))
                if layer=='foreground':depth[xx,yy]=(h-y)*32
        if name=='pine':foot=[(1,h-1),(2,h-1)]
        elif name=='gate':foot=[(x,y) for y in range(4,h) for x in range(w) if x<4 or x>=8]
        else:foot=[(x,y) for y in range(h-(4 if name=='shelter' else 2),h) for x in range(w)]
        for x,y in foot:
            if inside(x0+x,y0+y) and ((w-1-x if flip else x),y) in opaque:
                block(x0+x,y0+y,name);cells.append([x0+x,y0+y])
        placements.append(dict(object=name,position=[x0,y0],flip_horizontal=flip,collision=cells))
    for i,(name,x,y) in enumerate(config['props']):place(name,x,y,i%3==2 and name!='gate')
    bridges=[]
    if 'stone_bridge' in config:
        x0,y0=config['stone_bridge'];atlas,cols,(sx,sy,w,h),opaque=SPECS['bridge']
        for y in range(h):
            for x in range(w):
                if (x,y) in opaque:put('architecture',x0+x,y0+y,atlas_gid(atlas,(sy+y)*cols+sx+x))
                # Higher camera: three ground-level deck rows, shallow parapets.
                if y in (2,3,4):
                    reasons.pop((x0+x,y0+y),None)
                elif y in (1,5):block(x0+x,y0+y,'stone bridge parapet')
        bridges.append(dict(kind='stone',deck=[[x,y0+y] for y in (2,3,4) for x in range(x0,x0+w)]))
    if 'rope_bridge' in config:
        x0,x1,cy=config['rope_bridge']
        gap=set(range(config['rope_break'][0],config['rope_break'][1]+1))
        for y in range(cy-1,cy+2):
            for x in range(x0,x1+1):
                if x in gap:
                    block(x,y,'broken bridge gap')
                    continue
                col=0 if x==x0 else 2 if x==x1 else 1
                put('architecture',x,y,refs['floor']+45+(y-cy+1)*16+col)
                reasons.pop((x,y),None)
                if y!=cy:block(x,y,'rope bridge rail')
        atlas,columns,(sx,sy,w,h),opaque=SPECS['broken_rope_end']
        for tip_x,flip in [(min(gap)-w,False),(max(gap)+1,True)]:
            for y in range(h):
                for x in range(w):
                    sc=w-1-x if flip else x
                    gid=atlas_gid(atlas,(sy+y)*columns+sx+sc,flip)
                    put('architecture',tip_x+x,cy-1+y,gid)
            # Stop before the splintered inner tile, whose deck is incomplete.
            block(tip_x if flip else tip_x+w-1,cy,'broken bridge end')
        deck=[[x,cy] for x in range(x0,x1+1) if (x,cy) not in reasons]
        bridges.append(dict(kind='broken_rope',deck=deck,gap=[[x,cy] for x in sorted(gap)],
                            position=[x0,cy-1],size=[x1-x0+1,3]))
    bx,by=config['board'];put('decoration',bx,by,refs['outdoor']+12);block(bx,by,'trail board')
    boxes=root.find("objectgroup[@name='item_boxes']")
    originals={o.get('name'):o for o in boxes.findall('object')}
    assert set(originals)=={n for n,_,_,_ in config['chests']}
    for name,oid,x,y in config['chests']:
        assert not a['decoration'][y*W+x] and not a['foreground'][y*W+x],('chest overlap',name)
        obj=originals[name];assert int(obj.get('id'))==oid
        obj.set('x',str(x*32));obj.set('y',str((y+1)*32));block(x,y,'chest')
    for x,y in config['spawns']:
        assert not reasons.get((x,y)),('blocked patrol',segment,x,y,reasons.get((x,y)))
        put('spawn_tile',x,y,refs['outdoor']+47)
    for (x,y) in reasons:a['collision'][y*W+x]=refs['collision']
    if 'boss' in config:
        boss=root.find("objectgroup[@name='boss_enemy']/object")
        boss.set('x',str(config['boss'][0]*32));boss.set('y',str(config['boss'][1]*32))
    for obj in root.findall("objectgroup[@name='portals']/object"):
        entry=obj.get('name').startswith('entry_');cy=config['arrivals'][0 if entry else 1][1]
        geo=(0 if entry else W*32-12,(cy-2)*32,12,5*32)
        if entry:target=('town_04_frostholm',4,2) if segment=='01' else (PREFIX+f'{int(segment)-1:02}',*CONFIGS[int(segment)-2]['arrivals'][1])
        else:target=('town_05_ashenveil',19,29) if segment=='03' else (PREFIX+f'{int(segment)+1:02}',*CONFIGS[int(segment)]['arrivals'][0])
        for k,v in zip(['x','y','width','height'],geo):obj.set(k,str(v))
        props={p.get('name'):p for p in obj.findall('properties/property')}
        for k,v in zip(['target_map','target_position_x','target_position_y'],target):props[k].set('value',str(v))
    for layer in root.findall('layer'):
        name=layer.get('name');layer.set('width',str(W));layer.set('height',str(H))
        if name in ['collision','spawn_tile']:layer.set('visible','0')
        layer.find('data').text='\n'+',\n'.join(','.join(map(str,a[name][y*W:(y+1)*W])) for y in range(H))+'\n'
        if name=='foreground':
            props=layer.find('properties')
            if props is None:props=ET.Element('properties');layer.insert(0,props)
            props[:]=[]
            ET.SubElement(props,'property',name='sort_base_y_overrides',value=' '.join(f'{x},{y}:{v}' for (x,y),v in sorted(depth.items(),key=lambda v:(v[0][1],v[0][0]))))
    # Validate the authored tile route before publishing a regenerated map.
    seen={tuple(config['arrivals'][0])};q=deque(seen)
    while q:
        x,y=q.popleft()
        for n in [(x-1,y),(x+1,y),(x,y-1),(x,y+1)]:
            if inside(*n) and n not in reasons and n not in seen:seen.add(n);q.append(n)
    for pos in [*config['arrivals'],*config['spawns'],config.get('boss',config['arrivals'][0])]:assert tuple(pos) in seen,('unreachable',segment,pos)
    for name,_,x,y in config['chests']:
        assert any(p in seen for p in [(x-1,y),(x+1,y),(x,y-1),(x,y+1)]),('unreachable chest',name)
    for bridge in bridges:assert all(tuple(p) in seen for p in bridge['deck'])
    if 'npc' in config:
        x,y=config['npc']
        assert all(p in seen for p in [(x,y+1),(x+1,y+1),(x+1,y+2)]),('NPC blocked',segment)
    ET.indent(root,space=' ')
    p.write_text('<?xml version="1.0" encoding="UTF-8"?>\n'+ET.tostring(root,encoding='unicode').replace(' />','/>')+'\n')
    print(f'{p.stem}: {W}x{H}, {len(placements)} props, {len(config["spawns"])} patrols, {len(seen)} reachable cells')
    return dict(map=p.stem,size=[W,H],arrivals=config['arrivals'],npc_anchor=config.get('npc'),boss=config.get('boss'),
                board=config['board'],placements=placements,chests=[dict(name=n,id=i,position=[x,y]) for n,i,x,y in config['chests']],
                encounters=config['spawns'],bridges=bridges,reachable_cells=len(seen))


if __name__=='__main__':
    import argparse
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--segment',choices=['01','02','03'],help='Rebuild only this map and its layout record.')
    args=parser.parse_args()
    layout_path=SOURCE/'layouts.json'
    if args.segment:
        config=next(c for c in CONFIGS if c['segment']==args.segment)
        updated=build(config)
        reports=json.loads(layout_path.read_text())['maps']
        reports=[updated if r['map']==updated['map'] else r for r in reports]
    else:
        reports=[build(c) for c in CONFIGS]
    layout_path.write_text(json.dumps(dict(maps=reports),indent=2)+'\n')
