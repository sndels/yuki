# yuki

![screenshot](screenshot.png)

Ray cast renderer based on [Physically Based Rendering 3rd edition](http://www.pbr-book.org/) ([pbrt-v3](https://github.com/mmp/pbrt-v3)).

UI Features:
- Uniformly scaling film view
- Exposure control and ACES tonemap (Stephen Hill's fit)
- ~Non-blocking rendering
  - Relaunch on most setting changes
- Scene formats (partially) supported
  - [PLY](http://paulbourke.net/dataformats/ply/)
  - [Mitsuba 2.0](https://mitsuba2.readthedocs.io/en/latest/)
- EXR export for the raw pixels
  - [HDRView](https://github.com/wkjarosz/hdrview) is snappy for inspection and diffs

Renderer features:
- Tile-based rendering
  - Unwinding spiral pattern
  - Active work tiles are marked (but not separately cleared if render is restarted)
- BVH
- View rays only, direct lighting
- Stratified sampling
- Light types
  - Point
  - Spot

## yuki_derive

The math module is an excercise in new stuff, most notably proc_macros inspired by [derive_more](https://github.com/JelteF/derive_more). The implementation itself is quite specific to how the types are structured and supports non-conventional stuff like "deriving" math ops with scalar values with with other "vectors" of matching dimensions. The macro spaghetti is a overkill and likely more code than implementing the same stuff directly, especially if done through standard macros. But hey, it's cool I don't have to list component names for the impl :P

## License
While the main repo is licensed under MIT, parts of it are derived from projects licensed under different, compatible, terms. See LICENSES for details.
