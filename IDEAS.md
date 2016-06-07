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
* [ ] notification support (look how naughty does it)

## Short term stuff
* add helper methods using wrapper structs to avoid as many closures as
  possible in config.rs
* right now, the distribution of clients needs to be fixed in two ways:
  * first, client placement should be made explicit according to the new
    datastructures.
  * second, we need to decide upon the general paradigm of laziness considering
    client lists
* add proper error handling to `handle_map_request()` et al
