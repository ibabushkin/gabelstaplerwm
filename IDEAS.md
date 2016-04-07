# Concept draft
Main design aspect is the maximal independency of components: it should be
very simple to remove tags, bar(s), fancy layouts etc.
That way, users can choose components and replace/extend at will.

## General component ideas
* just tiling a specific area of the screen
* tiling algorithms are in their own modules, which are easily extended upon
  * layouts implement a trait that allows for proper interaction with the
    rest of the WM
* keybinding is done during WM init, i.e. passing an array of pairs to a function:
  * `wm.init_keys(&[(Key {mods: 64, key: 34}, some_function)]);`
  * a modal interface of sorts?
* additional functionality (mouse-replacement etc) is placed in additional modules
* tags work similarly to awesomeWM: N:M relationship between windows and tags
  * tags can be accessed by name, number etc.
  * rulesets decide upon the placement of clients
  * tag history or a similar concept to allow easy switching between sets of tags
* scratchpad or similar concept
* a simple bar/"privileged windows"
* notification support
