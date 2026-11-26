# colors.sh: one NAME=#hex line per palette key, for shell scripts to source.
# Run with: jq -r --arg n NAME -f shell-colors.jq palettes-map.json
.[$n] | to_entries[] | "\(.key)=\(.value)"
