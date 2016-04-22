# Concept draft
Main design aspect is the maximal independence of components: it should be
very simple to remove tags, bar(s), fancy layouts etc.
That way, users can choose components and replace/extend at will.

## General component ideas
* just tiling a specific area of the screen
  * a simple bar/"privileged windows"
* a modal interface of sorts? - numlock? Mode enum possible (user-defined)
* additional functionality (mouse-replacement etc) is placed in additional
  modules, that don't need to be compiled in (seperate crates?)
  * scratchpad or similar concept
* tags work similarly to awesomeWM: N:M relationship between windows and tags
  * rulesets decide upon the placement of clients
  * it's cheaper to delete them from history than perform all manipulations
    over it, so `Vec` is probably still the best idea
* helper util to get keycodes (via X I assume?)
* notification support (look how naughty does it)

## Short term goals
* make `window_system.rs` as modular and configurable as possible
  (Configurattion Struct or similar)
* Based on that, build a `config.rs` that we can then throughly test.
* Next, clean the code and make it more consistent to be more user-friendly
