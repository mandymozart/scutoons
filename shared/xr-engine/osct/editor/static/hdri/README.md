# HDRIs of the editor's stage

Six studio HDRIs from [Poly Haven](https://polyhaven.com) (CC0), 1k, the stage's sky and light (`backdrop: hdri`
in the toolbar's *view* group; `editor/src/lib/stage/preview.ts` `HDRIS`, drawn by `mock-stage.ts`):

| File | Source |
|---|---|
| `ferndale_studio_04_1k.hdr` | https://polyhaven.com/a/ferndale_studio_04 |
| `wooden_studio_07_1k.hdr` | https://polyhaven.com/a/wooden_studio_07 |
| `wooden_studio_13_1k.hdr` | https://polyhaven.com/a/wooden_studio_13 |
| `wooden_studio_14_1k.hdr` | https://polyhaven.com/a/wooden_studio_14 |
| `wooden_studio_17_1k.hdr` | https://polyhaven.com/a/wooden_studio_17 |
| `wooden_studio_19_1k.hdr` | https://polyhaven.com/a/wooden_studio_19 |

Another resolution (2k is about 6 MB a file):

```bash
curl -L -o ferndale_studio_04_1k.hdr https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/1k/ferndale_studio_04_1k.hdr
```

The desk's texture is Poly Haven's `plywood` (CC0, https://polyhaven.com/a/plywood), `../textures/`, tinted to a
screen-printed board in `mock-stage.ts`.
