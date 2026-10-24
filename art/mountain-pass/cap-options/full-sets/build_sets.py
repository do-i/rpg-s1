"""Normalize the remaining caps and assemble all three four-corner sets."""
from pathlib import Path
import hashlib
import json
import subprocess

SOURCE=Path(__file__).resolve().parent
ROOT=SOURCE.parents[3]
WORK=ROOT/'temp/mountain-pass/cap-options/full-sets'


def run(*args):
    subprocess.run(list(map(str,args)),check=True)


def rgba(path):
    return subprocess.check_output(['magick',str(path),'-depth','8','RGBA:-'])


def repair_upper_cap(target,style,corner):
    key=f'{style.lower()}-{corner}'
    metadata=SOURCE/f'{key}-join-repair.json'
    repair=json.loads(metadata.read_text())
    generated=SOURCE/repair['source']
    scaled=WORK/f'{key}-join-scaled.png'
    run('magick',generated,'-filter','point','-resize','32x32!',scaled)
    source=rgba(scaled)
    middle=rgba(SOURCE/repair['middle_reference'])
    original=rgba(target)
    assert original==rgba(SOURCE/repair['cap_reference']), 'repair anchor changed'
    result=bytearray(original)
    start,seam,middle_x=(28,31,0) if corner=='nw' else (0,0,31)
    for y in range(32):
        colors={middle[(y*32+x)*4:(y*32+x+1)*4] for x in range(32)
                if middle[(y*32+x)*4+3]}
        for x in range(start,start+4):
            i=(y*32+x)*4
            if not colors:
                color=b'\0'*4
            elif x==seam:
                j=(y*32+middle_x)*4
                color=middle[j:j+4]
            else:
                sample=source[i:i+4]
                color=min(sorted(colors),key=lambda c:sum((c[k]-sample[k])**2 for k in range(3)))
            result[i:i+4]=color
    assert all(result[(y*32+x)*4:(y*32+x+1)*4]==original[(y*32+x)*4:(y*32+x+1)*4]
               for y in range(32) for x in range(32) if not start<=x<start+4)
    assert all(result[(y*32+seam)*4:(y*32+seam+1)*4]==middle[(y*32+middle_x)*4:(y*32+middle_x+1)*4]
               for y in range(32))
    subprocess.run(['magick','-size','32x32','-depth','8','RGBA:-','-strip',
                    '-define','png:exclude-chunks=date,time',f'PNG32:{target}'],input=result,check=True)
    repair['source_sha256']=hashlib.sha256(generated.read_bytes()).hexdigest()
    repair['tile_sha256']=hashlib.sha256(target.read_bytes()).hexdigest()
    metadata.write_text(json.dumps(repair,indent=2)+'\n')


def main():
    import argparse
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--style',choices=['A','B','C'])
    args=parser.parse_args()
    WORK.mkdir(parents=True,exist_ok=True)
    prompts=json.loads((SOURCE/'prompts.json').read_text())
    sets=json.loads((SOURCE/'sets.json').read_text())
    reference_paths=[SOURCE/f'native-{c}.png' for c in sets['corner_order']]
    approved=[SOURCE.parent/f'option-{s.lower()}.png' for s in sets['sets']]
    approved_prompts=json.loads((SOURCE.parent/'prompts.json').read_text())
    for option in approved_prompts['options']:
        tile=SOURCE.parent/option['tile']
        assert hashlib.sha256(tile.read_bytes()).hexdigest()==option['tile_sha256'], 'approved southwest cap changed'
    palette=WORK/'palette.png'
    run('magick',*reference_paths,*approved,'+append','-unique-colors',palette)
    reference_raw={c:rgba(SOURCE/f'native-{c}.png') for c in sets['corner_order']}
    reports=[]
    for request in prompts['requests']:
        if args.style and request['style']!=args.style:continue
        corner=request['corner'];ref=SOURCE/f'native-{corner}.png'
        target=SOURCE/request['tile'];generated=SOURCE/request['source']
        x=28 if corner=='nw' else 0
        seam=WORK/f'seam-{corner}.png'
        run('magick',ref,'-crop',f'4x32+{x}+0','+repage',seam)
        run('magick',generated,'-channel','A','-threshold','50%','+channel',
            '-trim','+repage','-filter','point','-resize','32x32!',
            '+dither','-remap',palette,'-channel','A','-threshold','50%','+channel',
            '-background','black','-alpha','background',f'PNG32:{target}')
        run('magick',target,seam,'-geometry',f'+{x}+0',
            '-define','compose:outside-overlay=false','-compose','Copy','-composite',
            '-strip','-define','png:exclude-chunks=date,time',f'PNG32:{target}')
        repaired=corner in ('nw','ne')
        if repaired:
            repair_upper_cap(target,request['style'],corner)
        raw=rgba(target);old=reference_raw[corner]
        assert len(raw)==32*32*4 and set(raw[3::4])<={0,255}
        for y in range(32):
            i=(y*32+x)*4
            if not repaired:
                assert raw[i:i+16]==old[i:i+16],(request['style'],corner,'connector')
        request['source_sha256']=hashlib.sha256(generated.read_bytes()).hexdigest()
        request['tile_sha256']=hashlib.sha256(target.read_bytes()).hexdigest()
        reports.append(dict(style=request['style'],corner=corner,size=[32,32],
                            exact_connector=True,binary_alpha=True))
        print(f"PASS {request['style']} {corner}: 32x32, binary alpha, exact connector")
    panels=[]
    for style,definition in sets['sets'].items():
        if args.style and style!=args.style:continue
        label=WORK/f'{style.lower()}-set.png'
        cmd=['magick','-size','192x48','canvas:none']
        raw_tiles=[]
        for col,corner in enumerate(sets['corner_order']):
            p=SOURCE/definition['tiles'][corner]
            raw=rgba(p);assert len(raw)==32*32*4 and set(raw[3::4])<={0,255}
            x=28 if corner in ('nw','sw') else 0
            old=reference_raw[corner]
            if corner in ('nw','ne'):
                middle=rgba(SOURCE/f'{style.lower()}-{corner}-middle-reference.png')
                seam,middle_x=(31,0) if corner=='nw' else (0,31)
                assert all(raw[(y*32+seam)*4:(y*32+seam+1)*4]==middle[(y*32+middle_x)*4:(y*32+middle_x+1)*4]
                           for y in range(32))
            else:
                assert all(raw[(y*32+x)*4:(y*32+x+4)*4]==old[(y*32+x)*4:(y*32+x+4)*4]
                           for y in range(32)),(style,corner,'connector')
            raw_tiles.append(raw)
            cmd += [str(p),'-geometry',f'+{8+col*48}+8','-compose','Over','-composite']
        assert len(set(raw_tiles))==4,(style,'duplicate corners')
        run(*cmd,'-scale','768x192',label)
        panel=WORK/f'{style.lower()}-panel.png'
        cmd=['magick','-size','820x285','canvas:#151923',str(label),'-geometry','+26+65',
             '-compose','Over','-composite','-fill','#f1f3f6','-font','DejaVu-Sans',
             '-pointsize','24','-gravity','North','-annotate','+0+15',
             f"{style} — {definition['name']}"+(' — Zone 06' if style==sets['active_for_zone06'] else ''),
             '-pointsize','16','-gravity','NorthWest']
        for col,corner in enumerate(sets['corner_order']):
            cmd += ['-annotate',f'+{110+col*192}+260',corner.upper()]
        run(*cmd,panel)
        panels.append(panel)
    run('magick',*panels,'-append',WORK/(f'{args.style.lower()}-set-preview.png' if args.style else 'all-sets.png'))
    prompts['normalization']={'size':[32,32],'alpha':'binary','palette':'native cap references plus approved southwest tiles',
                              'preserved_connector':'NW/NE use their per-style join-repair.json files; southwest/east preserve native four-column connectors'}
    (SOURCE/'prompts.json').write_text(json.dumps(prompts,indent=2)+'\n')
    (WORK/(f'validation-{args.style.lower()}.json' if args.style else 'validation.json')).write_text(json.dumps(reports,indent=2)+'\n')


if __name__=='__main__':
    main()
