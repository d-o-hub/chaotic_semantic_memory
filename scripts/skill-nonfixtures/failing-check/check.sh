#!/usr/bin/env bash
# Fixture check script: always fails so the validator must propagate the
# non-zero exit status. It also appends one line per invocation so the
# harness can assert the runner executed it EXACTLY ONCE.
echo "check-run" >> "$(dirname "$0")/runs.log"
exit 1