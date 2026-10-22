from pathlib import Path
import subprocess as S,xml.etree.ElementTree as E,json

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
ASSETS=ROOT/'temp/ancient-ruins-border-export'
ROWS=ASSETS/'rows';ROWS.mkdir(parents=True,exist_ok=True)
TILES=ROOT/'assets/scenarios/rusted_kingdoms/media/tilesets'
def magick(*args):S.run(['magick',*map(str,args)],check=True)

# Find the three isolated generated sections using alpha only; all raster
# changes below are deterministic ImageMagick crops, resizes and composites.
source=OUT/'generated-modular-palisade.png'
width,height=map(int,S.check_output(['magick','identify','-format','%w %h',str(source)]).split())
alpha=S.check_output(['magick',str(source),'-depth','8','RGBA:-'])[3::4]
runs=[]
for x in range(width):
 if not any(alpha[y*width+x]>127 for y in range(height)):continue
 if not runs or x>runs[-1][-1]+1:runs.append([x])
 else:runs[-1].append(x)
runs=[run for run in runs if len(run)>10]
assert len(runs)==3
for name,run in zip(['north','body','south'],runs):
 magick(source,'-crop',f'{run[-1]-run[0]+1}x{height}+{run[0]}+0','+repage','-channel','A','-threshold','50%','+channel','-trim','+repage','-filter','point','-resize','18x96!','+dither','-colors','96','-channel','A','-threshold','50%','+channel',f'PNG32:{ASSETS}/{name}-normalized.png')

# Keep the generated grain, but give both repeating bodies a true constant
# 18px cross-section. Process scanlines with ImageMagick, never repaint pixels.
for name,start in [('body-a',16),('body-b',48)]:
 rows=[]
 for y in range(32):
  path=ROWS/f'{name}-{y:02d}.png'
  magick(ASSETS/'body-normalized.png','-crop',f'18x1+0+{start+y}','+repage','-trim','+repage','-filter','point','-resize','18x1!','-alpha','opaque',f'PNG32:{path}')
  rows.append(path)
 magick(*rows,'-append',ASSETS/f'{name}-band.png')
 magick('-size','32x32','canvas:none',ASSETS/f'{name}-band.png','-geometry','+7+0','-composite',f'PNG32:{ASSETS/name}.png')
for name,source,y in [('north-cap','north',0),('south-cap','south',64)]:
 magick(ASSETS/f'{source}-normalized.png','-crop',f'18x32+0+{y}','+repage',ASSETS/f'{name}-band.png')
 magick('-size','32x32','canvas:none',ASSETS/f'{name}-band.png','-geometry','+7+0','-composite',f'PNG32:{ASSETS/name}.png')

# Every allowed join shares exactly the same RGBA edge row.
magick(ASSETS/'body-a.png','-crop','32x1+0+16','+repage',ASSETS/'join.png')
for name,edges in [('north-cap',[31]),('body-a',[0,31]),('body-b',[0,31]),('south-cap',[0])]:
 args=[ASSETS/f'{name}.png']
 for y in edges:args += [ASSETS/'join.png','-geometry',f'+0+{y}','-composite']
 magick(*args,f'PNG32:{ASSETS/name}.png')

magick(ASSETS/'north-cap.png',ASSETS/'body-a.png',ASSETS/'body-b.png',ASSETS/'south-cap.png','+append','+dither','-colors','96','-channel','A','-threshold','50%','+channel',f'PNG32:{TILES}/ancient_ruins_borders.png')
tsx=E.Element('tileset',version='1.10',tiledversion='1.12.2',name='ancient_ruins_borders',tilewidth='32',tileheight='32',columns='4',tilecount='4')
E.SubElement(tsx,'image',source='ancient_ruins_borders.png',width='128',height='32');E.indent(tsx,space=' ')
(TILES/'ancient_ruins_borders.tsx').write_text('<?xml version="1.0" encoding="UTF-8"?>\n'+E.tostring(tsx,encoding='unicode').replace(' />','/>')+'\n')

raw=S.check_output(['magick',str(TILES/'ancient_ruins_borders.png'),'-depth','8','RGBA:-'])
tiles=[b''.join(raw[(y*128+x*32)*4:(y*128+(x+1)*32)*4] for y in range(32)) for x in range(4)]
assert set(raw[3::4])=={0,255}
bounds=[]
for i,tile in enumerate(tiles):
 xs=[x for y in range(32) for x in range(32) if tile[(y*32+x)*4+3]]
 assert (min(xs),max(xs))==(7,24),(i,min(xs),max(xs))
 bounds.append([7,24])
 for y in range(32):
  if i in (1,2):assert [tile[(y*32+x)*4+3] for x in range(32)]==[255 if 7<=x<=24 else 0 for x in range(32)]
row=lambda i,y:tiles[i][y*128:(y+1)*128]
for top,bottom in [(0,1),(0,2),(1,1),(1,2),(2,1),(2,2),(1,3),(2,3)]:assert row(top,31)==row(bottom,0),(top,bottom)
(OUT/'tile-validation.json').write_text(json.dumps({'tile_size':32,'opaque_width':18,'opaque_x_bounds':bounds,'middle_rows_constant_width':True,'all_allowed_vertical_rgba_joins_identical':True,'binary_alpha':True},indent=2)+'\n')
print('PASS: four 32x32 tiles, identical 18px widths, constant-width middle rows, eight exact RGBA joins and binary alpha.')
