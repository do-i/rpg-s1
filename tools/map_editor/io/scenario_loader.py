from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import yaml


@dataclass(frozen=True)
class ScenarioMaps:
    scenario_root: Path
    maps_dir: Path
    data_maps_dir: Path
    tmx_paths: list[Path]

    def yaml_for(self, tmx_path: Path) -> Path | None:
        """Return the map-data YAML matching a TMX file, or None if absent."""
        candidate = self.data_maps_dir / f"{tmx_path.stem}.yaml"
        return candidate if candidate.is_file() else None


def load_scenario_maps(scenario_root: Path) -> ScenarioMaps:
    manifest_path = scenario_root / "manifest.yaml"
    if not manifest_path.is_file():
        raise ValueError(
            f"Scenario manifest not found: {manifest_path}. "
            f"Expected a 'manifest.yaml' file at the scenario root "
            f"(example: rusted_kingdoms/manifest.yaml)."
        )

    with manifest_path.open("r", encoding="utf-8") as f:
        manifest = yaml.safe_load(f)

    refs = manifest.get("refs")
    if not isinstance(refs, dict) or "maps" not in refs:
        raise ValueError(
            f"Manifest {manifest_path} is missing required property 'refs.maps' "
            f"(example: refs:\\n  maps: data/maps/)."
        )

    maps_rel = refs["maps"]
    data_maps_dir = (scenario_root / maps_rel).resolve()

    # Where the TMX files live, most authoritative first. A package that declares `refs.tmx`
    # states its own layout and is believed; `assets/maps` is the historical location, kept so
    # older packages still load; `data/maps` is the last resort for packages that put both
    # kinds of file together.
    candidates = []
    tmx_rel = refs.get("tmx")
    if isinstance(tmx_rel, str) and tmx_rel:
        candidates.append((scenario_root / tmx_rel).resolve())
    candidates.append((scenario_root / "assets" / "maps").resolve())
    candidates.append(data_maps_dir)

    # A directory that exists but holds no TMX is not the map directory -- silently accepting
    # one is how a renamed asset tree turned into an editor that opened with zero maps and no
    # error to say why.
    tmx_dir = next(
        (path for path in candidates if path.is_dir() and any(path.glob("*.tmx"))),
        None,
    )
    if tmx_dir is None:
        tried = ", ".join(str(path) for path in candidates)
        raise ValueError(
            f"No directory containing .tmx files was found: tried {tried}. "
            f"Declare the TMX location as 'refs.tmx' in the scenario manifest."
        )

    tmx_paths = sorted(tmx_dir.glob("*.tmx"))
    return ScenarioMaps(
        scenario_root=scenario_root.resolve(),
        maps_dir=tmx_dir,
        data_maps_dir=data_maps_dir,
        tmx_paths=tmx_paths,
    )
