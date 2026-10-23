"""Normalize retained alpine sources and pack complete object groups."""
from pathlib import Path
import hashlib
import json
import subprocess
import xml.etree.ElementTree as ET

SOURCE = Path(__file__).resolve().parent
ROOT = SOURCE.parents[1]
WORK = ROOT / 'temp/mountain-foothills/export'
DEST = ROOT / 'assets/scenarios/rusted_kingdoms/media/tilesets'
WIDTH, HEIGHT = 864, 544


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
        subprocess.run([
            'magick', str(source), '-channel', 'A', '-threshold', '50%', '+channel',
            '-trim', '+repage', '-filter', 'point', '-resize', f'{width*32}x{height*32}',
            '-gravity', 'South', '-background', 'none', '-extent', f'{width*32}x{height*32}',
            '+dither', '-colors', '128', '-channel', 'A', '-threshold', '50%', '+channel',
            f'PNG32:{WORK}/{name}.png',
        ], check=True)
        asset['source_sha256'] = hashlib.sha256(source.read_bytes()).hexdigest()
        command.extend([str(WORK / f'{name}.png'), '-geometry', f'+{x*32}+{y*32}', '-composite'])
    image = DEST / 'mountain_foothills.png'
    subprocess.run(command + [f'PNG32:{image}'], check=True)
    tileset = ET.Element('tileset', version='1.10', tiledversion='1.12.2',
                         name='mountain_foothills', tilewidth='32', tileheight='32',
                         tilecount='459', columns='27')
    ET.SubElement(tileset, 'image', source=image.name, width=str(WIDTH), height=str(HEIGHT))
    ET.indent(tileset, space=' ')
    image.with_suffix('.tsx').write_text('<?xml version="1.0" encoding="UTF-8"?>\n' +
        ET.tostring(tileset, encoding='unicode').replace(' />', '/>') + '\n')
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
    print('PASS: 864x544; six complete objects, clear gutters, binary alpha.')


if __name__ == '__main__':
    main()
