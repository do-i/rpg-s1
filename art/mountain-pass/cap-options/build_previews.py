"""Normalize three southwest-cap proposals and render isolated Tiled previews."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import xml.etree.ElementTree as ET

SOURCE = Path(__file__).resolve().parent
ROOT = SOURCE.parents[2]
MEDIA = ROOT/'assets/scenarios/rusted_kingdoms/media'
WORK = ROOT/'temp/mountain-pass/cap-options'


def run(*args):
    subprocess.run(list(map(str,args)),check=True)


def rgba(path):
    return subprocess.check_output(['magick',str(path),'-depth','8','RGBA:-'])


def main():
    WORK.mkdir(parents=True,exist_ok=True)
    data=json.loads((SOURCE/'prompts.json').read_text())
    layout=json.loads((SOURCE.parent/'tile_layout.json').read_text())
    cols=layout['columns'];cap=layout['bridge_sets']['south']['tiles']['left']
    cx,cy=cap%cols*32,cap//cols*32
    palette=WORK/'palette.png';seam=WORK/'east-seam.png'
    reference=SOURCE/'southwest-cap-reference.png'
    run('magick',reference,SOURCE/'lower-middle-reference.png','+append','-unique-colors',palette)
    run('magick',reference,'-crop','4x32+28+0','+repage',seam)
    original_map=MEDIA/'maps/zone_06_mountain_pass_02.tmx'
    baseline=WORK/'baseline.png'
    run('tmxrasterizer',original_map,baseline)
    baseline_raw=rgba(baseline)
    width=84*32
    panels=[];reports=[]
    for option in data['options']:
        key=option['option'].lower();target=SOURCE/option['tile'];generated=SOURCE/option['source']
        run('magick',generated,'-channel','A','-threshold','50%','+channel',
            '-trim','+repage','-filter','point','-resize','32x32!',
            '+dither','-remap',palette,'-channel','A','-threshold','50%','+channel',
            '-background','black','-alpha','background',f'PNG32:{target}')
        run('magick',target,seam,'-geometry','+28+0','-define','compose:outside-overlay=false',
            '-compose','Copy','-composite','-strip','-define','png:exclude-chunks=date,time',f'PNG32:{target}')
        raw=rgba(target);old=rgba(reference)
        assert len(raw)==32*32*4 and set(raw[3::4])<={0,255}
        assert all(raw[(y*32+28)*4:(y+1)*32*4]==old[(y*32+28)*4:(y+1)*32*4] for y in range(32))
        folder=WORK/key;folder.mkdir(exist_ok=True)
        atlas=folder/'atlas.png'
        run('magick',MEDIA/'tilesets/mountain_pass.png',target,'-geometry',f'+{cx}+{cy}',
            '-define','compose:outside-overlay=false','-compose','Copy','-composite',f'PNG32:{atlas}')
        tileset=ET.parse(MEDIA/'tilesets/mountain_pass.tsx').getroot()
        tileset.find('image').set('source','atlas.png')
        ET.indent(tileset,space=' ')
        (folder/'atlas.tsx').write_text('<?xml version="1.0" encoding="UTF-8"?>\n'+ET.tostring(tileset,encoding='unicode')+'\n')
        root=ET.parse(original_map).getroot()
        for ts in root.findall('tileset'):
            source=ts.get('source')
            ts.set('source','atlas.tsx' if source.endswith('/mountain_pass.tsx') else os.path.relpath((original_map.parent/source).resolve(),folder))
        ET.indent(root,space=' ')
        tmx=folder/'map.tmx';tmx.write_text('<?xml version="1.0" encoding="UTF-8"?>\n'+ET.tostring(root,encoding='unicode')+'\n')
        run('tmxrasterizer',tmx,folder/'map.png')
        rendered=rgba(folder/'map.png');changes=0
        for i in range(0,len(rendered),4):
            if rendered[i:i+4]!=baseline_raw[i:i+4]:
                y,x=divmod(i//4,width)
                assert 34*32<=x<35*32 and 29*32<=y<30*32,(key,'change outside southwest cap',x,y)
                changes+=1
        run('magick',folder/'map.png','-crop','320x160+992+864','+repage','-scale','640x320',folder/'context.png')
        run('magick',target,'-scale','160x160',folder/'cap-large.png')
        panel=folder/'panel.png'
        run('magick','-size','680x600','canvas:#151923',folder/'cap-large.png','-geometry','+260+70',
            '-compose','Over','-composite',folder/'context.png','-geometry','+20+255','-composite',
            '-fill','#f1f3f6','-font','DejaVu-Sans','-pointsize','25','-gravity','North',
            '-annotate','+0+20',f"{option['option']}  {option['name']}",'-pointsize','15',
            '-annotate','+0+232','32x32 tile above / southwest corner in map below',panel)
        panels.append(panel)
        option['source_sha256']=hashlib.sha256(generated.read_bytes()).hexdigest()
        option['tile_sha256']=hashlib.sha256(target.read_bytes()).hexdigest()
        reports.append(dict(option=option['option'],changed_pixels=changes,changes_outside_southwest_cap=0,
                            exact_east_connector=True,dimensions=[32,32],binary_alpha=True))
        print(f"PASS {option['option']}: 32x32, exact east seam, {changes} changed pixels confined to southwest cap.")
    run('magick',*panels,'+append',WORK/'comparison.png')
    (SOURCE/'prompts.json').write_text(json.dumps(data,indent=2)+'\n')
    (WORK/'validation.json').write_text(json.dumps(reports,indent=2)+'\n')


if __name__=='__main__':
    main()
