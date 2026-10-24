"""Normalize retained mountain pass landmarks and pack complete object groups."""
from pathlib import Path
import hashlib
import json
import subprocess
import xml.etree.ElementTree as ET

from atlas_layout import pack_tiles

SOURCE = Path(__file__).resolve().parent
ROOT = SOURCE.parents[1]
WORK = ROOT / 'temp/mountain-pass/export'
DEST = ROOT / 'assets/scenarios/rusted_kingdoms/media/tilesets'
WIDTH, HEIGHT = 576, 576


def rgba(path):
    return subprocess.check_output(['magick', str(path), '-depth', '8', 'RGBA:-'])


def main():
    WORK.mkdir(parents=True, exist_ok=True)
    prompt_file = SOURCE / 'prompts.json'
    prompts = json.loads(prompt_file.read_text())
    command = ['magick', '-size', f'{WIDTH}x{HEIGHT}', 'canvas:none']
    for asset in prompts['assets']:
        name = asset['name']
        source = SOURCE / asset['source']
        x, y, width, height = asset['atlas_rect']
        if asset.get('normalization') == 'fixed-canvas-source-palette-left-seam':
            reference = SOURCE / asset['reference']
            target = WORK / f'{name}.png'
            seam = WORK / 'rope-seam.png'
            subprocess.run([
                'magick', str(source), '-filter', 'point', '-resize', '64x96!',
                '+dither', '-remap', str(reference), '-channel', 'A', '-threshold', '50%',
                '+channel', f'PNG32:{target}',
            ], check=True)
            subprocess.run(['magick', str(reference), '-crop', '16x96+0+0', '+repage',
                            f'PNG32:{seam}'], check=True)
            subprocess.run(['magick', str(target), str(seam), '-geometry', '+0+0',
                            '-define', 'compose:outside-overlay=false', '-compose', 'Copy',
                            '-composite', f'PNG32:{target}'], check=True)
            actual, original = rgba(target), rgba(reference)
            assert all(actual[y*64*4:y*64*4+16*4] == original[y*64*4:y*64*4+16*4]
                       for y in range(96)), 'broken bridge connection profile changed'
            asset['reference_sha256'] = hashlib.sha256(reference.read_bytes()).hexdigest()
        elif asset.get('normalization') == 'high-angle-bridge':
            # Preserve the 18x7 ID footprint, with a broad deck in rows 2-4.
            subprocess.run([
                'magick', str(source), '-channel', 'A', '-threshold', '50%', '+channel',
                '-trim', '+repage', '-filter', 'point', '-resize', '576x160!',
                '+dither', '-colors', '128', '-channel', 'A', '-threshold', '50%', '+channel',
                '-background', 'none', '-gravity', 'North', '-splice', '0x32',
                '-extent', '576x224', f'PNG32:{WORK}/{name}.png',
            ], check=True)
            # Leave the deck open to the native trail layer below the generated rails.
            # The source terrain pixels remain in their canonical atlas.
            subprocess.run([
                'magick', str(WORK/f'{name}.png'), '-alpha', 'on',
                '-region', '576x114+0+54', '-channel', 'A', '-evaluate', 'set', '0',
                '+channel', '+region', '-background', 'black', '-alpha', 'background',
                f'PNG32:{WORK}/{name}.png',
            ], check=True)
        else:
            subprocess.run([
                'magick', str(source), '-channel', 'A', '-threshold', '50%', '+channel',
                '-trim', '+repage', '-filter', 'point', '-resize', f'{width*32}x{height*32}',
                '-gravity', 'South', '-background', 'none', '-extent', f'{width*32}x{height*32}',
                '+dither', '-colors', '128', '-channel', 'A', '-threshold', '50%', '+channel',
                f'PNG32:{WORK}/{name}.png',
            ], check=True)
        asset['source_sha256'] = hashlib.sha256(source.read_bytes()).hexdigest()
        command.extend([str(WORK / f'{name}.png'), '-geometry', f'+{x*32}+{y*32}', '-composite'])
    image = WORK / 'object-layout.png'
    subprocess.run(command + ['-strip', '-define', 'png:exclude-chunks=date,time', f'PNG32:{image}'], check=True)
    prompt_file.write_text(json.dumps(prompts, indent=2) + '\n')
    raw = rgba(image)
    assert set(raw[3::4]) == {0, 255}
    for asset in prompts['assets']:
        x, y, w, h = asset['atlas_rect']
        block = subprocess.check_output(['magick', str(image), '-crop', f'{w*32}x{h*32}+{x*32}+{y*32}',
                                        '+repage', '-depth', '8', 'RGBA:-'])
        assert block == rgba(WORK / (asset['name']+'.png')), asset['name']
    for y in range(HEIGHT):
        for x in range(WIDTH):
            if not any(sx*32 <= x < (sx+w)*32 and sy*32 <= y < (sy+h)*32
                       for sx, sy, w, h in (a['atlas_rect'] for a in prompts['assets'])):
                assert raw[(y*WIDTH+x)*4+3] == 0
    bridge_recipe=json.loads((SOURCE/'bridge_tiles.json').read_text())
    cap_source=SOURCE/'cap-options/full-sets'
    subprocess.run(['python3',str(cap_source/'build_sets.py')],check=True)
    cap_manifest=json.loads((cap_source/'sets.json').read_text())
    decorative_sets={style:dict(name=definition['name'],tiles={corner:rgba(cap_source/path)
                    for corner,path in definition['tiles'].items()})
                    for style,definition in cap_manifest['sets'].items()}
    placement=json.loads((SOURCE/'atlas_placement.json').read_text())
    packed, layout = pack_tiles(raw,bridge_recipe,decorative_sets,cap_manifest['active_for_zone06'],placement)
    image = DEST / 'mountain_pass.png'
    width, height = layout['columns']*32, layout['rows']*32
    # Small palettes can use PNG8 without changing decoded pixels.
    palette_size=len({packed[i:i+4] for i in range(0,len(packed),4)})
    encoding='PNG8' if palette_size<=256 else 'PNG32'
    layout['png_encoding']='indexed8' if encoding=='PNG8' else 'rgba8'
    subprocess.run([
        'magick', '-size', f'{width}x{height}', '-depth', '8', 'RGBA:-',
        '+dither', '-define', 'png:compression-level=9',
        '-strip', '-define', 'png:exclude-chunks=date,time', f'{encoding}:{image}',
    ], input=packed, check=True)
    assert rgba(image) == packed
    tileset = ET.Element('tileset', version='1.10', tiledversion='1.12.2',
                         name='mountain_pass', tilewidth='32', tileheight='32',
                         tilecount=str(layout['tilecount']), columns=str(layout['columns']))
    ET.SubElement(tileset, 'image', source=image.name, width=str(width), height=str(height))
    for style,definition in layout['cap_sets'].items():
        for corner,tile_id in definition['tiles'].items():
            tile=ET.SubElement(tileset,'tile',id=str(tile_id))
            props=ET.SubElement(tile,'properties')
            ET.SubElement(props,'property',name='desc',value=f"Bridge {style} - {definition['name']} - {corner.upper()} cap")
    ET.indent(tileset, space=' ')
    image.with_suffix('.tsx').write_text('<?xml version="1.0" encoding="UTF-8"?>\n' +
        ET.tostring(tileset, encoding='unicode').replace(' />', '/>') + '\n')
    (SOURCE/'tile_layout.json').write_text(json.dumps(layout, indent=2)+'\n')
    print(f"PASS: {width}x{height}; {layout['tilecount']} slots, "
          f"{layout['unique_tiles']} unique tiles; 10 bridge variants, rope seam preserved.")


if __name__ == '__main__':
    main()
