"""Pack whole objects, reusable parapets and three four-corner cap families."""
from tile_transforms import transform_tile

TILE = 32
SOURCE_COLUMNS = 18
SOURCE_ROWS = 18


def extract_tiles(raw, columns, rows):
    width = columns * TILE
    return {
        row * columns + col: b"".join(
            raw[((row * TILE + y) * width + col * TILE) * 4:
                ((row * TILE + y) * width + (col + 1) * TILE) * 4]
            for y in range(TILE)
        )
        for row in range(rows)
        for col in range(columns)
    }


def pack_tiles(raw, bridge_recipe, decorative_sets, active_cap_set, placement):
    tiles = extract_tiles(raw, SOURCE_COLUMNS, SOURCE_ROWS)
    columns, rows = 14, 10
    width = columns * TILE
    packed = bytearray(width * rows * TILE * 4)
    canonical = {}
    mapping = {}
    bridge_sets = {}
    exact_duplicates = 0

    def store(tile, x, y):
        nonlocal exact_duplicates
        if not any(tile[3::4]):
            return None
        target = y * columns + x
        if tile in canonical:
            exact_duplicates += 1
            return canonical[tile]
        canonical[tile] = target
        for dy in range(TILE):
            start = ((y * TILE + dy) * width + x * TILE) * 4
            packed[start:start + TILE * 4] = tile[dy * TILE * 4:(dy + 1) * TILE * 4]
        return target

    def place(source_id, x, y):
        target = store(tiles[source_id], x, y)
        if target is not None:
            mapping[str(source_id)] = target

    # Keep the complete gateway together, including its transparent opening.
    for y in range(9):
        for x in range(12):
            place((9 + y) * SOURCE_COLUMNS + x, x, y)
    # Preserve the distinct upper/lower masonry, with caps and three snow levels.
    for band, (side, source_start) in enumerate((('north', 18), ('south', 90))):
        recipe = bridge_recipe[side]
        assert set(recipe['tiles']) == {'left', 'right', 'bare', 'light_snow', 'heavy_snow'}
        assert len(recipe['middle_pattern']) == 16
        ids = {}
        for index, (name, (sx, sy, w, h)) in enumerate(recipe['tiles'].items()):
            assert w == h == TILE
            tile = b''.join(raw[((sy+y)*576+sx)*4:((sy+y)*576+sx+TILE)*4]
                            for y in range(TILE))
            if name not in ('left', 'right'):
                offset = recipe.get('middle_offset_y', 0)
                assert 0 <= offset < TILE
                if offset:
                    # Move only fully clear bottom rows above the existing stonework.
                    split = (TILE-offset)*TILE*4
                    assert not any(tile[split:]), 'vertical alignment would crop visible pixels'
                    tile = tile[split:] + tile[:split]
                body_rows = range(4, 20) if side == 'north' else range(12, 30)
                assert all(tile[(y*TILE+x)*4+3] == 255
                           for y in (row+offset for row in body_rows)
                           for x in range(TILE)), 'gap in repeatable masonry'
            ids[name] = store(tile, 12+index % 2, band*3+index//2)
        assert len(set(ids.values())) == 5
        sequence = ['left', *recipe['middle_pattern'], 'right']
        assert set(recipe['middle_pattern']) == {'bare', 'light_snow', 'heavy_snow'}
        for x, name in enumerate(sequence):
            mapping[str(source_start+x)] = ids[name]
        bridge_sets[side] = dict(tiles=ids, sequence=sequence,
                                middle_offset_y=recipe.get('middle_offset_y', 0))
    for y in range(3):
        for x in range(2):
            place((9 + y) * SOURCE_COLUMNS + 12 + x, 12 + x, 6 + y)

    cap_sets = {}
    for set_index, (style, definition) in enumerate(decorative_sets.items()):
        ids = {}
        for corner_index, corner in enumerate(('nw', 'ne', 'sw', 'se')):
            tile = definition['tiles'][corner]
            assert len(tile) == TILE*TILE*4 and set(tile[3::4]) <= {0, 255}
            ids[corner] = store(tile, set_index*4+corner_index, 9)
        assert len(set(ids.values())) == 4
        cap_sets[style] = dict(name=definition['name'], tiles=ids)
    assert active_cap_set in cap_sets
    active = cap_sets[active_cap_set]['tiles']
    for side, source_start, left, right in [('north', 18, 'nw', 'ne'), ('south', 90, 'sw', 'se')]:
        definition = bridge_sets[side]
        ids = definition['tiles']
        definition['original_caps'] = {name: ids[name] for name in ('left', 'right')}
        ids['left'], ids['right'] = active[left], active[right]
        for x, name in enumerate(definition['sequence']):
            mapping[str(source_start+x)] = ids[name]

    visible = {i for i, tile in tiles.items() if any(tile[3::4])}
    assert visible == {int(i) for i in mapping}, "unpacked visible source tile"
    exported = extract_tiles(packed, columns, rows)
    bridge_ids = {*range(18, 36), *range(90, 108)}
    assert all(tiles[int(old)] == exported[new] for old, new in mapping.items()
               if int(old) not in bridge_ids)
    assert set(bridge_sets['north']['tiles'].values()).isdisjoint(
        bridge_sets['south']['tiles'].values())
    layout = {
        "source_columns": SOURCE_COLUMNS,
        "source_rows": SOURCE_ROWS,
        "columns": columns,
        "rows": rows,
        "tilecount": columns * rows,
        "visible_source_tiles": len(visible),
        "unique_tiles": len(canonical),
        "exact_duplicates_reused": exact_duplicates,
        "bridge_source_tiles": 36,
        "bridge_reusable_tiles": 10,
        "bridge_sets": bridge_sets,
        "active_cap_set": active_cap_set,
        "cap_sets": cap_sets,
        "tile_ids": dict(sorted(mapping.items(), key=lambda item: int(item[0]))),
    }
    return apply_placement(bytes(packed), layout, placement)


def apply_placement(raw, layout, placement):
    """Use the owner's sample layout after assembling the stable source cuts."""
    tiles = extract_tiles(raw, layout['columns'], layout['rows'])
    columns, rows = placement['columns'], placement['rows']
    packed = bytearray(columns * rows * TILE * TILE * 4)
    targets = set()
    entries = placement['placements']
    visible = {i for i, tile in tiles.items() if any(tile[3::4])}
    assert visible - {int(i) for i in entries} == set(placement['removed_visible_ids'])
    for old, entry in entries.items():
        target = entry['new_id']
        assert 0 <= target < columns * rows and target not in targets
        targets.add(target)
        tile = transform_tile(tiles[int(old)], entry['transform'])
        for y in range(TILE):
            start = ((target // columns * TILE + y) * columns * TILE + target % columns * TILE) * 4
            packed[start:start + TILE * 4] = tile[y * TILE * 4:(y + 1) * TILE * 4]
    for definition in layout['bridge_sets'].values():
        definition.pop('original_caps')
        definition['tiles'] = {name: entries[str(old)]['new_id'] for name, old in definition['tiles'].items()}
    for definition in layout['cap_sets'].values():
        definition['tiles'] = {name: entries[str(old)]['new_id'] for name, old in definition['tiles'].items()}
    layout['tile_transforms'] = {source: entries[str(old)]['transform'] for source, old in layout['tile_ids'].items()
                                 if entries[str(old)]['transform']}
    layout['tile_ids'] = {source: entries[str(old)]['new_id'] for source, old in layout['tile_ids'].items()}
    retained = [tile for tile in extract_tiles(packed, columns, rows).values() if any(tile[3::4])]
    assert len(retained) == len(set(retained)) == len(visible) - len(placement['removed_visible_ids'])
    layout.update(columns=columns, rows=rows, tilecount=columns * rows,
                  unique_tiles=len(retained),
                  placement='atlas_placement.json', removed_plain_cap_source_ids=placement['removed_visible_ids'])
    return bytes(packed), layout
