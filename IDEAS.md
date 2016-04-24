# Concept draft
Main design aspect is the maximal independence of components: it should be
very simple to remove tags, bar(s), fancy layouts etc.
That way, users can choose components and replace/extend at will.

## General component ideas
* just tiling a specific area of the screen
  * a simple bar/"privileged windows"
* a modal interface of sorts - Mode enum
* additional functionality is placed in additional modules, that don't need
  to be compiled in (seperate crates)
  * scratchpad or similar concept
  * mouse replacement
  * bar
* tags work similarly to awesomeWM: N:M relationship between windows and tags
  * rulesets decide upon the placement of clients
  * it's cheaper to delete them from history than perform all manipulations
    over it, so `Vec` is probably still the best idea
* helper util to get keycodes (via X I assume?)
* notification support (look how naughty does it)

## Short term goals
* clean the code even more ;)
* find a way to avoid as many calls to `arrange_windows()` as possible
  * maybe make callback closures return values of an enum that gets interpreted
    accordingly - do we need to redraw? do we need to add a new client etc?
    This would also make it possible to avoid passing mutable references, making
    the code more functional and safe
* maybe find a better way than having 3(!) functions for focus setting in
  different places
