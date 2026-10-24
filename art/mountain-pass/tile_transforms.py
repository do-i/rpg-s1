"""Bake Tiled square-tile flips and compensate map flags after atlas packing."""
from functools import lru_cache

HFLIP = 0x80000000
VFLIP = 0x40000000
DFLIP = 0x20000000
FLAGS = tuple(h | v | d for h in (0, HFLIP) for v in (0, VFLIP) for d in (0, DFLIP))


def transform_tile(tile, flags, size=32):
    assert flags in FLAGS and len(tile) == size * size * 4
    result = bytearray(len(tile))
    for y in range(size):
        for x in range(size):
            dx, dy = (y, x) if flags & DFLIP else (x, y)
            if flags & HFLIP:
                dx = size - 1 - dx
            if flags & VFLIP:
                dy = size - 1 - dy
            source, target = (y * size + x) * 4, (dy * size + dx) * 4
            result[target:target + 4] = tile[source:source + 4]
    return bytes(result)


@lru_cache(maxsize=64)
def compensate_flags(original, baked):
    """Return flags that preserve the original view of a transformed atlas tile."""
    marker = b''.join(bytes((i, 0, 0, 255)) for i in range(4))
    target = transform_tile(marker, original, 2)
    moved = transform_tile(marker, baked, 2)
    return next(flags for flags in FLAGS if transform_tile(moved, flags, 2) == target)
