# yuki

Ray cast renderer based on [pbrt3](http://www.pbr-book.org/) ([source](https://github.com/mmp/pbrt-v3)).

Math lib is an excercise in new stuff, most notably proc_macros inspired by [derive_more](https://github.com/JelteF/derive_more). The implementation itself is quite specific to how my vector types are structured and supports "deriving" ops with scalar values as well as with other "vectors" of matching dimensions. The whole thing macro spaghetti is a overkill and likely more code than implementing the same stuff directly, especially if done through standard macros. But hey, it's cool I don't have to list component names for the impl :D

UI is based on [imgui-rs](https://github.com/imgui-rs/imgui-rs) and the older [gfx-rs](https://github.com/gfx-rs/gfx/tree/pre-ll).
