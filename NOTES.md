client:
  window (and maybe another unique id)
  geometry
  mapping
  properties
  tags

tagset:
  unique id
  set of tags
  layout

screen:
  unique id
  geometry

tag_tree:
  container arena
  root container id
  focused container id
  selected/marked container id

container:
  types: hsplit/vsplit/tab/window
  parent
  children
  last focused

arena:
  map<window, client>
  map<set of tags, tag_tree>
  map<screen, tagset>
