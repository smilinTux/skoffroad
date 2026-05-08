# Material Attribution

All terrain materials shipped under `assets/materials/` are licensed
**[CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/)**
(public domain) and re-distributable under any terms, including the GPL-3.0
license of this game.

We retain attribution as a courtesy, not as a license requirement.

## Sources

| Slot  | Source                          | Asset ID  | License | Resolution |
|-------|---------------------------------|-----------|---------|------------|
| dirt  | [ambientCG](https://ambientcg.com/view?id=Ground054) | `Ground054` | CC0 | 1K JPG |
| grass | [ambientCG](https://ambientcg.com/view?id=Grass001)  | `Grass001`  | CC0 | 1K JPG |
| rock  | [ambientCG](https://ambientcg.com/view?id=Rock047)   | `Rock047`   | CC0 | 1K JPG |
| mud   | [ambientCG](https://ambientcg.com/view?id=Ground068) | `Ground068` | CC0 | 1K JPG |

ambientCG is operated by Lennart Demes (Struffel Productions).
Special thanks for releasing thousands of high-quality scans into the
public domain.

## Re-fetching / changing resolution

`scripts/fetch_materials.sh` is idempotent — it downloads only missing
assets. Pass `--force` to redownload, or edit the `ASSETS` array to
swap asset IDs / change resolution (`1K` → `2K` etc).

```sh
./scripts/fetch_materials.sh           # fill in any missing materials
./scripts/fetch_materials.sh --force   # redownload everything
```

## Files we keep

For each material we keep three maps and rename them:

| File             | Source map name (ambientCG) | Used as |
|------------------|------------------------------|---------|
| `albedo.jpg`     | `<id>_<res>-JPG_Color.jpg`     | base color |
| `normal.jpg`     | `<id>_<res>-JPG_NormalGL.jpg`  | normal map (GL convention) |
| `roughness.jpg`  | `<id>_<res>-JPG_Roughness.jpg` | roughness |

Other maps shipped in the source archives (displacement, AO, metalness)
are dropped to keep repository size down. Bevy's PBR can synthesize AO
from normals at acceptable quality; we revisit if visuals call for it.
