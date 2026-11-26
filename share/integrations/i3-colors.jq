# i3 colour variables: one "set $<key> <colour>" line for each palette key
.[$n] | to_entries[] | "set $\(.key) \(.value)"
