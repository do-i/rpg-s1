"""Export the retained sanctuary props as complete 32-pixel object groups."""
from pathlib import Path
import hashlib
import json
import subprocess
import xml.etree.ElementTree as ET

SOURCE = Path(__file__).resolve().parent
ROOT = SOURCE.parents[1]
WORK = ROOT / 'temp/ancient-ruins-sanctum/export'
DEST = ROOT / 'assets/scenarios/rusted_kingdoms/media/tilesets'
OBJECTS = {'altar': [0, 0, 10, 8], 'sarcophagus': [12, 5, 4, 3]}


def main():
    WORK.mkdir(parents=True, exist_ok=True)
    prompt_file = SOURCE / 'prompts.json'
    prompts = json.loads(prompt_file.read_text())
    for asset in prompts['assets']:
        name = asset['name']
        source = SOURCE / asset['source']
        _, _, width, height = OBJECTS[name]
        subprocess.run([
            'magick', str(source), '-channel', 'A', '-threshold', '50%', '+channel',
            '-trim', '+repage', '-filter', 'point', '-resize', f'{width*32}x{height*32}',
            '-gravity', 'South', '-background', 'none', '-extent', f'{width*32}x{height*32}',
            '+dither', '-colors', '128', '-channel', 'A', '-threshold', '50%', '+channel',
            f'PNG32:{WORK}/{name}.png',
        ], check=True)
        asset['source_sha256'] = hashlib.sha256(source.read_bytes()).hexdigest()
    command = ['magick', '-size', '512x256', 'canvas:none']
    for name, (x, y, _, _) in OBJECTS.items():
        command.extend([str(WORK / f'{name}.png'), '-geometry', f'+{x*32}+{y*32}', '-composite'])
    image = DEST / 'ancient_ruins_sanctum.png'
    subprocess.run(command + [f'PNG32:{image}'], check=True)
    tileset = ET.Element('tileset', version='1.10', tiledversion='1.12.2',
                         name='ancient_ruins_sanctum', tilewidth='32', tileheight='32',
                         tilecount='128', columns='16')
    ET.SubElement(tileset, 'image', source=image.name, width='512', height='256')
    ET.indent(tileset, space=' ')
    image.with_suffix('.tsx').write_text('<?xml version="1.0" encoding="UTF-8"?>\n' +
                                        ET.tostring(tileset, encoding='unicode').replace(' />', '/>') + '\n')
    prompt_file.write_text(json.dumps(prompts, indent=2) + '\n')
    raw = subprocess.check_output(['magick', str(image), '-depth', '8', 'RGBA:-'])
    assert set(raw[3::4]) == {0, 255}
    for name, (x, y, w, h) in OBJECTS.items():
        block = subprocess.check_output(['magick', str(image), '-crop', f'{w*32}x{h*32}+{x*32}+{y*32}',
                                        '+repage', '-depth', '8', 'RGBA:-'])
        original = subprocess.check_output(['magick', str(WORK/f'{name}.png'), '-depth', '8', 'RGBA:-'])
        assert block == original, name
    for y in range(256):
        for x in range(512):
            if not any(sx*32 <= x < (sx+w)*32 and sy*32 <= y < (sy+h)*32
                       for sx, sy, w, h in OBJECTS.values()):
                assert raw[(y*512+x)*4+3] == 0
    print('PASS: 512x256, 128 slots, complete objects preserved, clear gutters, binary alpha.')


if __name__ == '__main__':
    main()
