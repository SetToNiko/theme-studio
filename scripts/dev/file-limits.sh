#!/usr/bin/env bash
# Usage: scripts/dev/file-limits.sh [--list] [DIR]
#        scripts/dev/file-limits.sh --selftest
# Prints each .rs file under DIR (default: studio/src) that has more than 500
# production lines of code, as "<lines> <path>". With --list, it prints each
# file, also the files under the limit. --selftest counts a sample file with
# each difficult case and compares the count with the known value.
#
# Production code is the part of the file before the "#[cfg(test)]" line that
# is followed by "mod tests". Files named tests.rs, and files in a tests/
# folder, hold only tests and are skipped.
#
# A line of code is a line with code on it. These lines do not count: empty
# lines, lines with only spaces, lines with only a line comment (//, ///, //!),
# and the lines of a /* ... */ block comment. A line with code and a comment
# counts as one line. A non-empty line inside a string literal is code.
#
# The awk program reads each character, because "//" and "/*" in a string
# literal do not start a comment.
#
# Exit status: 1 if a file is over the limit, else 0. The limit is in the
# project-conventions skill: 500 lines of code without tests.
set -euo pipefail

# The self-test. Each line of the sample has its count in the comment of this
# list (1 = a line of code):
#   header 0, use 1, empty 0, doc 0, block comment 0 + 0, string start 1,
#   "//" in the string 1, empty line in the string 0, "/*" in the string 1,
#   char literal with a quote 1, lifetimes 1, raw string with "//" 1, s 1,
#   closing brace 1, cfg(test) on a function 1, the function 1.
#   Then "#[cfg(test)]" + "mod tests" stops the count. Total: 11.
if [ "${1:-}" = "--selftest" ]; then
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    cat > "$tmp/sample.rs" <<'SAMPLE'
//! The header.
use std::fmt; // a comment after code

/// A doc comment.
/* A block comment
   that ends here. */
const A: &str = "
// not a comment: this line is in the string

/* not a comment either */";
const Q: char = '"'; // the quote is a char literal
fn f<'a>(s: &'a str) -> &'a str {
    let _r = r#"say "hi" // in a raw string"#;
    s
}
#[cfg(test)]
fn helper() {}
#[cfg(test)]
mod tests {
    fn t() {}
}
SAMPLE
    got=$(bash "$0" --list "$tmp" | cut -d' ' -f1)
    if [ "$got" != 11 ]; then
        echo "file-limits self-test failed: expected 11 lines of code, got $got. Fix the awk program." >&2
        exit 1
    fi
    echo "file-limits self-test: ok (11 lines of code)"
    exit 0
fi

limit=500
list=0
if [ "${1:-}" = "--list" ]; then
    list=1
    shift
fi
dir="${1:-studio/src}"
over=0

while IFS= read -r -d '' file; do
    lines=$(awk '
        # Stop at "mod tests" when the line before it was "#[cfg(test)]".
        pending && /^[[:space:]]*mod tests/ { exit }

        # The "#[cfg(test)]" line before this line was not for "mod tests":
        # it is code.
        pending { code++; pending = 0 }
        !in_string && !in_block && /^#\[cfg\(test\)\]/ { pending = 1; next }

        {
            # A line that starts inside a string literal is code, but an empty
            # line never counts (the kit definition of a line of code).
            has_code = in_string && $0 !~ /^[[:space:]]*$/
            line = $0
            n = length(line)
            for (i = 1; i <= n; i++) {
                c = substr(line, i, 1)
                two = substr(line, i, 2)
                if (in_block) {
                    if (two == "*/") { in_block = 0; i++ }
                    continue
                }
                if (in_string) {
                    if (raw_end != "") {
                        if (substr(line, i, length(raw_end)) == raw_end) {
                            in_string = 0; i += length(raw_end) - 1
                        }
                    } else if (c == "\\") {
                        i++
                    } else if (c == "\"") {
                        in_string = 0
                    }
                    continue
                }
                if (two == "//") break
                if (two == "/*") { in_block = 1; i++; continue }
                if (c ~ /[[:space:]]/) continue
                has_code = 1

                # A raw string: r"...", r#"..."#, br#"..."#, and so on.
                if (c == "r" && match(substr(line, i), /^r#*"/)) {
                    hashes = RLENGTH - 2
                    raw_end = "\"" substr("################", 1, hashes)
                    in_string = 1; i += RLENGTH - 1
                    continue
                }
                if (c == "\"") { in_string = 1; raw_end = ""; continue }

                # A char literal (for example, the quote char or an escape).
                # A lifetime such as the "a" in &\047a str has no closing quote.
                if (c == "\047") {
                    if (substr(line, i + 1, 1) == "\\" && match(substr(line, i), /^\047\\.[^\047]*\047/)) {
                        i += RLENGTH - 1
                    } else if (substr(line, i + 2, 1) == "\047") {
                        i += 2
                    }
                }
            }
            if (has_code) code++
        }

        END { print code + 0 }
    ' "$file")
    if [ "$list" = 1 ]; then
        echo "$lines $file"
    fi
    if [ "$lines" -gt "$limit" ]; then
        if [ "$list" = 0 ]; then
            echo "$lines $file"
        fi
        over=1
    fi
done < <(find "$dir" -name '*.rs' ! -name tests.rs ! -path "*/tests/*" -print0 | sort -z)

exit "$over"
