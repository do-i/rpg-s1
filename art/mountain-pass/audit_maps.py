"""Audit saved Zone 06 TMX navigation using the runtime's 20x18 foot rectangle."""
from collections import deque
from pathlib import Path
import json
import math
import re
import subprocess
import xml.etree.ElementTree as ET

ROOT=Path(__file__).resolve().parents[2]
MEDIA=ROOT/'assets/scenarios/rusted_kingdoms/media'
PREFIX='zone_06_mountain_pass_'
MP_LAYOUT=json.loads((Path(__file__).resolve().parent/'tile_layout.json').read_text())
MP_TILE_IDS=MP_LAYOUT['tile_ids']

def props(obj):return {p.get('name'):p.get('value') for p in obj.findall('properties/property')}
def adjacent(p):
    x,y=p
    return [(x-1,y),(x+1,y),(x,y-1),(x,y+1)]
def flood(start,walkable):
    assert start in walkable,('blocked start',start)
    seen={start};queue=deque(seen)
    while queue:
        for p in adjacent(queue.popleft()):
            if p in walkable and p not in seen:seen.add(p);queue.append(p)
    return seen

def audit():
    documents={p.stem:ET.parse(p).getroot() for p in (MEDIA/'maps').glob('*.tmx')}
    reports=[]
    for segment in ['01','02','03']:
        mid=PREFIX+segment;root=documents[mid];w,h=int(root.get('width')),int(root.get('height'))
        layers={l.get('name'):l for l in root.findall('layer')}
        values={n:[int(v) for v in l.find('data').text.split(',')] for n,l in layers.items()}
        assert all(len(v)==w*h for v in values.values())
        def rect_clear(x,y,rw,rh):
            if x<0 or y<0 or x+rw>w*32 or y+rh>h*32:return False
            return all(not values['collision'][cy*w+cx] for cy in range(math.floor(y/32),math.ceil((y+rh)/32))
                       for cx in range(math.floor(x/32),math.ceil((x+rw)/32)))
        walk={(x,y) for y in range(h) for x in range(w) if rect_clear(x*32+6,y*32+7,20,18)}
        incoming=[]
        for source,doc in documents.items():
            for portal in doc.findall("objectgroup[@name='portals']/object"):
                pp=props(portal)
                if pp.get('target_map')==mid:
                    target=(int(pp['target_position_x']),int(pp['target_position_y']))
                    assert target in walk,(mid,'arrival',source,target)
                    incoming.append((source,target))
        assert len(incoming)==2,(mid,incoming)
        seen=flood(incoming[0][1],walk)
        assert all(p in seen for _,p in incoming)
        # Compare important identities with the repository's pre-redesign map.
        old=ET.fromstring(subprocess.check_output(['git','show',f'HEAD:{(MEDIA / f"maps/{mid}.tmx").relative_to(ROOT)}']))
        assert [(c.tag,c.get('id'),c.get('name')) for c in root if c.tag in ['layer','objectgroup']]==[(c.tag,c.get('id'),c.get('name')) for c in old if c.tag in ['layer','objectgroup']]
        oldboxes={o.get('name'):(o.get('id'),props(o)) for o in old.findall("objectgroup[@name='item_boxes']/object")}
        boxes=root.findall("objectgroup[@name='item_boxes']/object")
        assert {o.get('name'):(o.get('id'),props(o)) for o in boxes}==oldboxes
        for o in boxes:
            x=int((float(o.get('x'))+16)//32);y=int((float(o.get('y'))-16)//32)
            assert values['collision'][y*w+x],(mid,'chest lacks collision',o.get('name'))
            assert any(p in seen for p in adjacent((x,y))),(mid,'chest approach',o.get('name'))
        markers={(i%w,i//w) for i,v in enumerate(values['spawn_tile']) if v}
        assert markers<=seen
        assert all(max(abs(x-a),abs(y-b))>=5 for x,y in markers for _,(a,b) in incoming),(mid,'patrol near arrival')
        for o in root.findall("objectgroup[@name='boss_enemy']/object"):
            p=(int(float(o.get('x'))//32),int(float(o.get('y'))//32))
            assert p in seen
            assert all(max(abs(x-p[0]),abs(y-p[1]))>=7 for x,y in markers),'patrol in boss court'
        for o in root.findall("objectgroup[@name='portals']/object"):
            x,y,pw,ph=(float(o.get(k,'0')) for k in ['x','y','width','height'])
            assert 0<=x<x+pw<=w*32 and 0<=y<y+ph<=h*32,(mid,'out of bounds portal')
            assert any(x<cx*32+26 and cx*32+6<x+pw and y<cy*32+25 and cy*32+7<y+ph for cx,cy in seen),(mid,'unreachable portal')
            pp=props(o);dest=documents[pp['target_map']]
            returns=[p for p in dest.findall("objectgroup[@name='portals']/object") if props(p).get('target_map')==mid]
            assert returns,(mid,'no return link')
            dx,dy=int(pp['target_position_x']),int(pp['target_position_y']);dw=int(dest.get('width'))
            dest_collision=[int(v) for v in dest.find("layer[@name='collision']/data").text.split(',')]
            assert not dest_collision[dy*dw+dx],(mid,'blocked destination')
        board_gid=next(int(t.get('firstgid'))+12 for t in root.findall('tileset') if t.get('source').endswith('/outdoor.tsx'))
        boards=[(i%w,i//w) for i,v in enumerate(values['decoration']) if v==board_gid]
        assert len(boards)==1 and any(p in seen for p in adjacent(boards[0]))
        assert layers['collision'].get('visible')==layers['spawn_tile'].get('visible')=='0'
        # All empty ground remains usable; hidden blockers may not split off snowfields.
        assert seen==walk,(mid,'disconnected walkable cells',len(walk-seen))
        if segment=='01':
            metadata=(ROOT/f'assets/scenarios/rusted_kingdoms/data/maps/{mid}.yaml').read_text()
            nx,ny=map(int,re.search(r'position: \[(\d+), (\d+)\]',metadata).groups())
            assert rect_clear(nx*32+22,ny*32+41,20,18),'NPC foot rectangle intersects scenery'
            # Stand below the NPC's actual feet, rather than on its YAML anchor.
            assert (nx+1,ny+2) in seen
        if segment=='02':
            refs={Path(t.get('source')).stem:int(t.get('firstgid')) for t in root.findall('tileset')}
            stone={(x,y) for y in range(26,29) for x in range(34,52) if values['terrain'][y*w+x]==794}
            rail_ids={refs['mountain_pass']+MP_TILE_IDS[str(v)] for v in [*range(18,36),*range(90,108)]}
            rails={(i%w,i//w) for i,v in enumerate(values['architecture']) if (v&0x1fffffff) in rail_ids}
            assert rails=={(x,y) for y in (25,29) for x in range(34,52)},'stone bridge parapets'
            assert not (rails&walk),'stone bridge rails lack collision'
            active_caps=MP_LAYOUT['cap_sets'][MP_LAYOUT['active_cap_set']]['tiles']
            assert MP_LAYOUT['bridge_sets']['north']['tiles']['left']==active_caps['nw']
            assert MP_LAYOUT['bridge_sets']['north']['tiles']['right']==active_caps['ne']
            assert MP_LAYOUT['bridge_sets']['south']['tiles']['left']==active_caps['sw']
            assert MP_LAYOUT['bridge_sets']['south']['tiles']['right']==active_caps['se']
            assert all(len(set(s['tiles'].values()))==4 for s in MP_LAYOUT['cap_sets'].values())
            used_sets=[]
            for side,y in [('north',25),('south',29)]:
                definition=MP_LAYOUT['bridge_sets'][side]
                ids=definition['tiles']
                sequence=[refs['mountain_pass']+ids[name] for name in definition['sequence']]
                actual=[values['architecture'][y*w+x] for x in range(34,52)]
                assert actual==sequence,('bridge variant order',side)
                assert len(set(actual))==5 and len(set(actual[1:-1]))==3,('bridge variant count',side)
                used_sets.append(set(actual))
            assert len(used_sets[0]|used_sets[1])==10,'bridge must use two distinct five-tile sets'

            intact_rope={(i%w,i//w) for i,v in enumerate(values['architecture']) if refs['floor']+61<=v<=refs['floor']+63}
            torn_ids={refs['mountain_pass']+MP_TILE_IDS[str(v)] for v in [192,193]}
            torn_deck={(i%w,i//w) for i,v in enumerate(values['architecture']) if (v&0x1fffffff) in torn_ids}
            rope=(intact_rope|torn_deck)&walk
            assert stone=={(x,y) for y in range(26,29) for x in range(34,52)} and stone<=seen,'stone bridge alignment/deck'
            assert all(values['terrain'][y*w+x]==794 for y in range(26,29) for x in range(32,54)),'deck paving does not join both banks'
            assert rope=={(x,48) for x in range(30,52) if x not in range(39,43)},'broken rope approach decks'
            assert len(torn_deck-walk)==2,'torn plank ends must block movement'
            for y in range(47,50):
                for x in [40,41]:
                    assert not values['architecture'][y*w+x] and values['collision'][y*w+x],'gap needs clear art and solid collision'
            start,finish=incoming[0][1],incoming[1][1]
            assert finish in flood(start,walk-rope),'stone bridge cannot carry the route'
            assert finish not in flood(start,walk-stone),'broken bridge still connects the banks'
            left=flood((30,48),walk-stone);right=flood((51,48),walk-stone)
            assert (38,48) in left and (43,48) in right,'broken ends lack bank access'
            assert (43,48) not in left and (38,48) not in right,'broken ends connect across the gap'
        if segment=='03':
            assert all((x,y) in seen for x in range(56,60) for y in range(13,23)),'blocked gate opening'
        reports.append(dict(map=mid,size=[w,h],reachable_cells=len(seen),arrivals=incoming,
                            reachable_patrols=len(markers),reachable_chests=len(boxes),all_open_ground_connected=True))
        print(f'PASS {mid}: {len(seen)} connected foot positions; arrivals, portals, board, {len(boxes)} chests, {len(markers)} patrols')
    return reports

if __name__=='__main__':
    report=audit()
    out=ROOT/'temp/mountain-pass/navigation-audit.json';out.parent.mkdir(parents=True,exist_ok=True)
    out.write_text(json.dumps(report,indent=2)+'\n')
