# Font Attribution

All fonts shipped under `assets/fonts/` are licensed under the
**[SIL Open Font License 1.1](https://openfontlicense.org/open-font-license-official-text/)**
(OFL-1.1).  OFL fonts may be used, modified, and redistributed freely,
including bundled inside commercial software, provided the font files
themselves are not sold standalone.

## Sources

| File           | Family                 | Style           | Source                                                    | License |
|----------------|------------------------|-----------------|-----------------------------------------------------------|---------|
| `primary.ttf`  | JetBrains Mono         | Regular         | [github.com/JetBrains/JetBrainsMono](https://github.com/JetBrains/JetBrainsMono) | OFL-1.1 |
| `display.ttf`  | Inter Display          | Regular         | [github.com/rsms/inter](https://github.com/rsms/inter)    | OFL-1.1 |

### JetBrains Mono

Copyright 2020 The JetBrains Mono Project Authors
([github.com/JetBrains/JetBrainsMono](https://github.com/JetBrains/JetBrainsMono)).
Version 2.304, released 2023-01-14.

Used as `primary.ttf` — the monospace face for all HUD labels, gauges, and
data readouts.

### Inter / Inter Display

Copyright 2016 The Inter Project Authors
([github.com/rsms/inter](https://github.com/rsms/inter)).
Version 4.0, released 2023.

Used as `display.ttf` — the proportional display face for headers, menus, and
title text.

## Re-fetching

`scripts/fetch_fonts.sh` is idempotent — it downloads only missing fonts.
Pass `--force` to redownload everything.

```sh
./scripts/fetch_fonts.sh           # fill in any missing fonts
./scripts/fetch_fonts.sh --force   # redownload all fonts
```
