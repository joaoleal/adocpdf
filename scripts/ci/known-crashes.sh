#!/usr/bin/env bash
#
# Decides whether the panics a fuzz run reported are ones already recorded.
#
# `libfuzzer-sys` aborts before unwinding, so the guard that contains
# `asciidoc-parser`'s panics in the shipped renderer never runs under the
# fuzzer, and the job would otherwise fail every week on defects that are
# already guarded and already tested.
#
# **Matching is by panic location, not by input bytes.** An earlier version
# compared the reproducer against recorded bytes, and the very next fuzz run
# produced a fifty-byte variant of a crash already recorded — the same upstream
# defect reached by a different input. The set of inputs that reaches one defect
# is unbounded; the line it panics at is not.
#
# Three rules keep the allowlist from hiding a regression, and all are load
# bearing:
#
#   1. Only the locations in `fuzz/known-crashes.toml` are tolerated. A panic
#      anywhere else — including anywhere in this project's own crates — fails.
#   2. A crash reporting no panic location at all fails. That is what a timeout
#      or an out-of-memory looks like, and neither is recorded.
#   3. Every tolerated panic is printed with the reason from its entry, so a
#      green run still says what it found.
#
# The record is also read by `crates/adocpdf-asciidoc/tests/known_crashes.rs`,
# which asserts on the pinned stable toolchain that each entry's sample input is
# still refused by the guard. That is what stops an entry outliving the defect
# it names.
#
# Usage: scripts/ci/known-crashes.sh <fuzz-output-log> [artifacts-dir]
set -euo pipefail

RECORD="fuzz/known-crashes.toml"
LOG="${1:?usage: known-crashes.sh <fuzz-output-log> [artifacts-dir]}"
ARTIFACTS="${2:-fuzz/artifacts}"
readonly RECORD LOG ARTIFACTS

if [[ ! -f "$RECORD" ]]; then
  echo "error: $RECORD is missing; the job cannot tell a known defect from a new one" >&2
  exit 1
fi

if [[ ! -f "$LOG" ]]; then
  echo "error: no fuzz output at $LOG; there is nothing to classify" >&2
  exit 1
fi

# Flattens the record to one `location<TAB>name<TAB>reason` line per table.
read_record() {
  awk '
    /^[[:space:]]*\[/ {
      name = $0
      gsub(/^[[:space:]]*\[|\][[:space:]]*$/, "", name)
      next
    }
    /^[[:space:]]*(location|reason)[[:space:]]*=/ {
      key = $1
      value = $0
      sub(/^[^=]*=[[:space:]]*/, "", value)
      gsub(/^"|"$/, "", value)
      if (key == "location") { location = value } else { reason = value }
      if (location != "" && reason != "") {
        printf "%s\t%s\t%s\n", location, name, reason
        location = ""
        reason = ""
      }
    }
  ' "$RECORD"
}

# Every distinct location the run reported a panic at.
panic_locations() {
  grep -oE "panicked at [^ ]+:[0-9]+:[0-9]+" "$LOG" |
    sed 's/^panicked at //' |
    sort -u
}

crashed=0
if [[ -d "$ARTIFACTS" ]] && [[ -n "$(find "$ARTIFACTS" -type f -print -quit 2>/dev/null)" ]]; then
  crashed=1
fi

locations="$(panic_locations || true)"

if [[ -z "$locations" ]]; then
  if [[ "$crashed" -eq 1 ]]; then
    echo "error: the fuzzer produced a reproducer but reported no panic location." >&2
    echo "A timeout or an out-of-memory looks like this, and neither is recorded." >&2
    find "$ARTIFACTS" -type f >&2
    exit 1
  fi
  echo "no panic reported: nothing to classify"
  exit 0
fi

tolerated=0
unknown=()

while IFS= read -r reported; do
  matched=""

  while IFS=$'\t' read -r location name reason; do
    # Suffix match: the reported path carries whatever prefix cargo's registry
    # has on this machine, and an entry must not depend on it.
    if [[ "$reported" == *"$location"* ]]; then
      matched="$name"
      echo "tolerated: $name"
      echo "           at $reported"
      echo "           $reason"
      break
    fi
  done < <(read_record)

  if [[ -n "$matched" ]]; then
    tolerated=$((tolerated + 1))
  else
    unknown+=("$reported")
  fi
done <<< "$locations"

echo
echo "tolerated $tolerated known defect(s); ${#unknown[@]} not recorded"

if [[ ${#unknown[@]} -gt 0 ]]; then
  echo
  echo "error: the fuzzer found a panic that is not recorded:" >&2
  for reported in "${unknown[@]}"; do
    echo "  $reported" >&2
  done
  echo >&2
  echo "Minimise the reproducer, fix or guard the defect, add a regression test," >&2
  echo "and only then add it to $RECORD with the reason it is tolerated." >&2
  exit 1
fi
