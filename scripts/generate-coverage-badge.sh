#!/usr/bin/env sh
set -eu

lcov_file="${1:-lcov.info}"
badge_file="${2:-assets/coverage.svg}"

if [ ! -f "$lcov_file" ]; then
    echo "coverage input not found: $lcov_file" >&2
    exit 1
fi

summary="$(awk '
    /^LF:/ { total += substr($0, 4) }
    /^LH:/ { hit += substr($0, 4) }
    END {
        if (total == 0) {
            print "0 0 0.0";
        } else {
            printf "%d %d %.1f\n", hit, total, (hit * 100.0) / total;
        }
    }
' "$lcov_file")"

set -- $summary
hit="$1"
total="$2"
coverage="$3"

coverage_int="${coverage%.*}"
if [ "$coverage_int" -ge 90 ]; then
    color="#4c1"
elif [ "$coverage_int" -ge 75 ]; then
    color="#97ca00"
elif [ "$coverage_int" -ge 60 ]; then
    color="#dfb317"
else
    color="#e05d44"
fi

mkdir -p "$(dirname "$badge_file")"

cat > "$badge_file" <<EOF
<svg xmlns="http://www.w3.org/2000/svg" width="118" height="20" role="img" aria-label="coverage: ${coverage}%">
  <title>coverage: ${coverage}% (${hit}/${total} lines)</title>
  <linearGradient id="s" x2="0" y2="100%">
    <stop offset="0" stop-color="#bbb" stop-opacity=".1"/>
    <stop offset="1" stop-opacity=".1"/>
  </linearGradient>
  <clipPath id="r">
    <rect width="118" height="20" rx="3" fill="#fff"/>
  </clipPath>
  <g clip-path="url(#r)">
    <rect width="63" height="20" fill="#555"/>
    <rect x="63" width="55" height="20" fill="${color}"/>
    <rect width="118" height="20" fill="url(#s)"/>
  </g>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" font-size="11">
    <text x="31.5" y="15" fill="#010101" fill-opacity=".3">coverage</text>
    <text x="31.5" y="14">coverage</text>
    <text x="90.5" y="15" fill="#010101" fill-opacity=".3">${coverage}%</text>
    <text x="90.5" y="14">${coverage}%</text>
  </g>
</svg>
EOF

echo "Generated $badge_file for ${coverage}% coverage (${hit}/${total} lines)."
