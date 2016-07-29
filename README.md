# gabelstaplerwm
gabelstaplerwm is a semidynamic tiling window manager written in the Rust
programming language and using the XCB wrappers available. It's main design
goals are simplicity, correctness in behaviour and configurability, coupled
with compile-time configuration by the user, minimizing overhead at runtime
and allowing for sophisticated dynamic behaviour. This is achieved by minimizing
the responsibilities of the window manager.

## Concepts
gabelstaplerwm is inspired by dwm and awesomewm, adding a few ideas of it's own
to the concept of a dynamic window manager. It shares the tag idiom with it's
ancestors-in-spirit, but implements a slightly different approach at managing
them. The author believes this is the most flexible solution achieved using
such a system, while still efficient to implement and use.

### What are tags at all?
Users of awesome/dwm are familiar with the tag concept. Where classical
approaches to window management introduce a concept such as a workspace, tags
are a more flexible alternative. Essentially, each window is associated with
a set of tags, and a user can display an arbitrary set of tags, which would
render all windows tagged with at least one of those tags on the screen.

Naturally, this concept is tightly coupled with layouts: if the sets of windows
displayed on screen are computed dynamically, then manually arranging them in
a grid would be painful and suboptimal. Thus, the geometries of the windows shown
are computed dynamically by so-called layouts, which are simple arrangement
patterns that can be applied to arbitrary amounts of windows.

These concepts are battle-tested and implemented in a few so-called *dynamic*
window managers, of which awesome and dwm are the most prominent. However,
even if arbitrary sets of tags can be shown at any time, tags remain a very
flexible implementation of workspaces. This is because tag sets with more than
one element are hard to display in a quick fashion and remain second-class citizens
that way. gabelstaplerwm reduces tags to symbolic values and allows for powerful
mechanisms to add, remove, and edit tag sets at runtime, making working with the
concept more efficient.

### How do you use them?
This is a much more practical question, isn't it? At the time of writing, I haven't
yet decided whether to keep or modify the current handling and approach to tags.
Thus, this section remains empty for now. See the documentation section for more
information.

### Why doesn't it have feature X?
Frankly, I don't want to implement as much as possible, but to keep the codebase
as small and clean as possible (and I think this worked out so far). This means
there are no fancy graphics, no window decoration besides a border, no included bars,
no wallpaper setting functionality, no transparent bubbles indicating your battery
status and other cr*p I don't need.

And never *will* be.

If you need it, implement it, and use it to your liking, or use a different piece
of software. Extensibility is one of the main goals, compiling in additional crates
with extended features is being worked on, so this should be possible (if you deem it
worth you time).

## Configuration and Installation
Simple as the source itself:

1. Read the `src/wm/config.rs` file.
2. Read the other sources, as you see fit.
3. Edit it to your liking.
4. Compile and install with `cargo`.
5. Repeat as necessary.

## Documentation
Currently, the only docs available are the (partly pretty extensive) comments in
the sources. If there are unclear aspects, feel free to file an issue on GitHub.
There is also a help document planned, but considering the configuration model,
understanding the source is pretty useful anyway.

## Future Development
The project isn't stale, but I pause it from time to time when other things happen.
The things that are planned for the near or less-near future are all listed in the
[ideas file](https://www.github.com/ibabushkin/gabelstaplerwm/blob/master/IDEAS.md).

## Contributing
Contribution is always welcome, be it bug reports, feedback, patches or proposals.
GitHub should be an appropriate platform for this.
