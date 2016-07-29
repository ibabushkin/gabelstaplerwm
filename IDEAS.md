# Concept draft
Main design aspect is the maximal independence of components: it should be
possible to remove the modules built in or functionality one doesn't need.
This is orthogonal to providing an interface that is easy to build and extend
upon, which is realized using the callback closure system allowing for arbitrary
user-generated output.

## General component ideas
* additional functionality is placed in additional modules, that don't *need*
  to be compiled in, realized via separate crates
  * [ ] mouse replacement (being worked on)
  * [ ] bar (unlikely I will write this, but feel free)
  * [ ] notification support (look up how naughty does it and optionally implement)

## Short term stuff
* [ ] add proper error handling to `handle_map_request()` etc.
* [ ] finish / polish plugin system
