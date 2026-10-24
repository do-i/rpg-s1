"""Render all three installed cap families in independent copies of the map."""
from pathlib import Path
import json
import os
import subprocess
import xml.etree.ElementTree as ET

SOURCE=Path(__file__).resolve().parent
ROOT=SOURCE.parents[3]
MEDIA=ROOT/'assets/scenarios/rusted_kingdoms/media'
WORK=ROOT/'temp/mountain-pass/cap-options/full-sets'
POSITIONS={'nw':(34,25),'ne':(51,25),'sw':(34,29),'se':(51,29)}


def run(*args):
    subprocess.run(list(map(str,args)),check=True)


def rgba(path):
    return subprocess.check_output(['magick',str(path),'-depth','8','RGBA:-'])


def render(original,folder,tiles):
    folder.mkdir(parents=True,exist_ok=True)
    root=ET.parse(original).getroot()
    first=int(next(t.get('firstgid') for t in root.findall('tileset') if t.get('source').endswith('/mountain_pass.tsx')))
    data=root.find("layer[@name='architecture']/data")
    cells=[int(v) for v in data.text.split(',')];width=int(root.get('width'))
    for corner,(x,y) in POSITIONS.items():cells[y*width+x]=first+tiles[corner]
    data.text='\n'+',\n'.join(','.join(map(str,cells[y*width:(y+1)*width])) for y in range(int(root.get('height'))))+'\n'
    for ts in root.findall('tileset'):
        ts.set('source',os.path.relpath((original.parent/ts.get('source')).resolve(),folder))
    ET.indent(root,space=' ')
    tmx=folder/'map.tmx';tmx.write_text('<?xml version="1.0" encoding="UTF-8"?>\n'+ET.tostring(root,encoding='unicode')+'\n')
    run('tmxrasterizer',tmx,folder/'map.png')
    return folder/'map.png'

def main():
    layout=json.loads((SOURCE.parents[1]/'tile_layout.json').read_text())
    original=MEDIA/'maps/zone_06_mountain_pass_02.tmx'
    active=layout['cap_sets'][layout['active_cap_set']]['tiles']
    before=rgba(render(original,WORK/'baseline',active))
    width=int(ET.parse(original).getroot().get('width'))
    panels=[]
    for style,definition in layout['cap_sets'].items():
        folder=WORK/style.lower()
        render(original,folder,definition['tiles'])
        image=rgba(folder/'map.png');changes=0
        for i in range(0,len(image),4):
            if image[i:i+4]!=before[i:i+4]:
                y,x=divmod(i//4,width*32)
                assert any(cx*32<=x<(cx+1)*32 and cy*32<=y<(cy+1)*32 for cx,cy in POSITIONS.values()),(style,x,y)
                changes+=1
        run('magick',folder/'map.png','-crop','768x256+992+768','+repage',folder/'bridge.png')
        panel=folder/'panel.png'
        run('magick',folder/'bridge.png','-background','#151923','-gravity','North','-splice','0x42',
            '-fill','#f1f3f6','-font','DejaVu-Sans','-pointsize','23','-annotate','+0+10',
            f"{style} — {definition['name']}"+(' — Zone 06' if style==layout['active_cap_set'] else ''),panel)
        panels.append(panel)
        print(f'PASS {style}: all render changes confined to four cap cells ({changes} pixels).')
    run('magick',*panels,'-append',WORK/'bridges-comparison.png')


if __name__=='__main__':
    main()
