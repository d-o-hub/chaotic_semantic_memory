# LOC Check

```bash
for file in src/*.rs; do
  LOC=$(wc -l < "$file")
  if [ "$LOC" -gt 500 ]; then
    echo "FAIL $file exceeds 500 LOC ($LOC lines)"
    exit 1
  fi
  echo "OK $file: $LOC LOC"
done
```
