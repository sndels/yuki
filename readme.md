# yuki

![screenshot](screenshot.png)

Ray cast renderer mostly based on [Physically Based Rendering 3rd edition](http://www.pbr-book.org/) ([pbrt-v3](https://github.com/mmp/pbrt-v3)).

## Goals
- Explore offline rendering techniques
- Try out how Rust helps/complicates a moderately complex system
  - Including vectorization, at some point, hopefully...
- Prioritize interactivity
  - Shorter iteration times for scene, parameter tweaks mean more exploration and validation


## Features

### UI
- Uniformly scaling film view
- Tone mapping
  - Filmic ACES tonemap (Stephen Hill's fit) with exposure
  - Heatmap
    - Single channel or luminance
    - Dynamic fitting of min,max
- ~Non-blocking rendering
  - Relaunch on most relevant setting changes
- EXR export for the raw values or tone mapped output
  - [HDRView](https://github.com/wkjarosz/hdrview) is snappy for inspection and diffs


### Renderer
- Tile-based rendering
  - Unwinding spiral pattern
    - Camera controls usable with longer frame times than if rendered row-by-row
  - Active work tiles are marked
    - Not separately cleared when film clear is disabled to minimize lag
- BVH
- Integrator abstraction
  - Whitted for direct diffuse lighting
  - Debug integrators
- Stratified sampling
- Light types
  - Point
  - Spot

### Scene formats (partially) supported
  - [PLY](http://paulbourke.net/dataformats/ply/)
  - [Mitsuba 2.0](https://mitsuba2.readthedocs.io/en/latest/)

## yuki_derive

The math module is an excercise in new stuff, most notably proc_macros inspired by [derive_more](https://github.com/JelteF/derive_more). The implementation itself is quite specific to how the types are structured and supports non-conventional stuff like "deriving" math ops with scalar values or other "vectors" of matching dimensions.

The macro spaghetti is a overkill and likely more code than implementing the same stuff directly, especially if done through standard macros. But hey, it's cool I don't have to list component names for the impl macro :P

## License
While the main repo is licensed under MIT, parts of it are derived from projects licensed under different, compatible terms. See LICENSES for details.
