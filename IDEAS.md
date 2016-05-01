# Concept draft
Main design aspect is the maximal independence of components: it should be
very simple to remove tags, bar(s), fancy layouts etc.
That way, users can choose components and replace/extend at will.

## General component ideas
* additional functionality is placed in additional modules, that don't need
  to be compiled in (seperate crates)
  * scratchpad or similar concept
  * mouse replacement
  * bar
* tags work similarly to awesomeWM: N:M relationship between windows and tags
  * [ ] rulesets decide upon the placement of clients
  * it's cheaper to delete them from history than perform all manipulations
    over it, so `Vec` is probably still the best idea
* [ ] helper util to get keycodes (via X I assume?)
* [ ] notification support (look how naughty does it)
