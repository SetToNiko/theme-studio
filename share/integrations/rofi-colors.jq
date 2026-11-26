# raw colours only; config.rasi decides what uses them
"* {",
(.[$n] | to_entries[] | "    \(.key | gsub("_"; "-")): \(.value);"),
"}"
