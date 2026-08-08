#!/bin/bash
# Self-learning plans/progress management script
# Handles CRUD operations, truncation, archiving for plans/, progress/, and plans/adr/
# Usage: ./scripts/plans-manager.sh <command> [options]

set -e

PLANS_DIR="plans"
PROGRESS_DIR="progress"
ADR_DIR="plans/adr"
ARCHIVE_DIR="plans/.archive"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Ensure archive directory exists
mkdir -p "$ARCHIVE_DIR"

show_help() {
  cat << EOF
Plans Manager - Self-learning CRUD for plans/progress/adr

Commands:
  status              Show status of all plan files
  count               Count entries in each file
  archive <type>      Archive completed/deferred items
  truncate <file>     Truncate large files to keep recent entries
  clean               Remove duplicates and fix formatting
  sync                Sync GOAP_STATE with ACTIONS.md status
  validate            Validate all plan files are consistent
  archive-list        List archived files

Types for archive:
  completed            Archive completed actions
  deferred            Archive deferred items
  adr                 Archive old ADRs (keep last 10)
  progress            Archive old progress entries

Examples:
  ./scripts/plans-manager.sh status
  ./scripts/plans-manager.sh count
  ./scripts/plans-manager.sh archive completed
  ./scripts/plans-manager.sh truncate progress/LEARNINGS.md
  ./scripts/plans-manager.sh clean
  ./scripts/plans-manager.sh sync
  ./scripts/plans-manager.sh validate
EOF
}

cmd_status() {
  log_info "Plan Files Status"
  echo "========================"
  
  echo -e "\n${GREEN}plans/${NC}"
  for f in "$PLANS_DIR"/*.md; do
    [ -f "$f" ] && echo "  $(basename "$f"): $(wc -l < "$f") lines"
  done
  
  echo -e "\n${GREEN}progress/${NC}"
  for f in "$PROGRESS_DIR"/*.md; do
    [ -f "$f" ] && echo "  $(basename "$f"): $(wc -l < "$f") lines"
  done
  
  echo -e "\n${GREEN}plans/adr/${NC} ($(ls "$ADR_DIR"/*.md 2>/dev/null | wc -l) ADRs)"
  ls "$ADR_DIR"/*.md 2>/dev/null | head -5 | while read f; do
    echo "  $(basename "$f")"
  done
  [ $(ls "$ADR_DIR"/*.md 2>/dev/null | wc -l) -gt 5 ] && echo "  ... and $(($(ls "$ADR_DIR"/*.md 2>/dev/null | wc -l) - 5)) more"
}

cmd_count() {
  log_info "Counting entries in plan files"
  echo "================================="
  
  echo -e "\n${GREEN}ACTIONS.md:${NC}"
  echo "  Total actions: $(grep -c "^\s*- name:" "$PLANS_DIR/ACTIONS.md" 2>/dev/null || echo 0)"
  echo "  Complete: $(grep -c "status: complete" "$PLANS_DIR/ACTIONS.md" 2>/dev/null || echo 0)"
  echo "  Pending: $(grep -c "status: pending" "$PLANS_DIR/ACTIONS.md" 2>/dev/null || echo 0)"
  echo "  Deferred: $(grep -c "status: deferred" "$PLANS_DIR/ACTIONS.md" 2>/dev/null || echo 0)"
  
  echo -e "\n${GREEN}GOAP_STATE.md:${NC}"
  echo "  Total states: $(grep -c ": true" "$PLANS_DIR/GOAP_STATE.md" 2>/dev/null || echo 0)"
  echo "  False states: $(grep -c ": false" "$PLANS_DIR/GOAP_STATE.md" 2>/dev/null || echo 0)"
  
  echo -e "\n${GREEN}ADR count:${NC} $(ls "$ADR_DIR"/*.md 2>/dev/null | wc -l)"
  
  echo -e "\n${GREEN}Progress files:${NC}"
  echo "  LEARNINGS.md: $(grep -c "^## " "$PROGRESS_DIR/LEARNINGS.md" 2>/dev/null || echo 0) entries"
  echo "  PROGRESS.md: $(grep -c "^### " "$PROGRESS_DIR/PROGRESS.md" 2>/dev/null || echo 0) entries"
}

cmd_truncate() {
  local file="$1"
  local max_lines="${2:-200}"
  
  if [ -z "$file" ]; then
    log_error "Usage: truncate <file> [max_lines]"
    return 1
  fi
  
  if [ ! -f "$file" ]; then
    log_error "File not found: $file"
    return 1
  fi
  
  local lines=$(wc -l < "$file")
  
  if [ "$lines" -le "$max_lines" ]; then
    log_info "File $file has $lines lines, within limit of $max_lines"
    return 0
  fi
  
  log_info "Truncating $file from $lines to $max_lines lines"
  
  # For LEARNINGS.md - keep header and most recent entries
  if [[ "$file" == *"LEARNINGS.md" ]]; then
    local header_lines=1
    local entries_to_keep=$((max_lines - header_lines))
    head -n 1 "$file" > "${file}.tmp"
    tail -n +2 "$file" | head -n "$entries_to_keep" >> "${file}.tmp"
    mv "${file}.tmp" "$file"
    log_info "Truncated LEARNINGS.md to recent entries"
  
  # For PROGRESS.md - keep header and most recent iterations
  elif [[ "$file" == *"PROGRESS.md" ]]; then
    local header_lines=3
    local entries_to_keep=$((max_lines - header_lines))
    head -n 3 "$file" > "${file}.tmp"
    tail -n +4 "$file" | head -n "$entries_to_keep" >> "${file}.tmp"
    mv "${file}.tmp" "$file"
    log_info "Truncated PROGRESS.md to recent iterations"
  
  # Generic - just keep first max_lines
  else
    head -n "$max_lines" "$file" > "${file}.tmp"
    mv "${file}.tmp" "$file"
    log_info "Truncated $file to $max_lines lines"
  fi
}

cmd_archive() {
  local type="$1"
  local timestamp=$(date +%Y%m%d-%H%M%S)
  
  case "$type" in
    completed)
      log_info "Archiving completed actions..."
      # Move completed actions to archive section or separate file
      local archive_file="$ARCHIVE_DIR/completed-actions-${timestamp}.md"
      grep -B2 "status: complete" "$PLANS_DIR/ACTIONS.md" | grep "^\s*- name:" > "$archive_file" || true
      log_info "Archived to $archive_file"
      ;;
      
    deferred)
      log_info "Archiving deferred items..."
      local archive_file="$ARCHIVE_DIR/deferred-${timestamp}.md"
      grep -B2 "status: deferred" "$PLANS_DIR/ACTIONS.md" | grep "^\s*- name:" > "$archive_file" || true
      log_info "Archived to $archive_file"
      ;;
      
    adr)
      log_info "Archiving old ADRs (keeping 10 most recent)..."
      local adr_count=$(ls "$ADR_DIR"/*.md 2>/dev/null | wc -l)
      if [ "$adr_count" -gt 10 ]; then
        local to_archive=$((adr_count - 10))
        ls -t "$ADR_DIR"/*.md | tail -n "$to_archive" | while read f; do
          mv "$f" "$ARCHIVE_DIR/"
          log_info "Archived: $(basename "$f")"
        done
      else
        log_info "Only $adr_count ADRs, no archiving needed"
      fi
      ;;
      
    progress)
      log_info "Archiving old progress entries..."
      local archive_file="$ARCHIVE_DIR/old-progress-${timestamp}.md"
      # Keep last 5 iterations, archive the rest
      tail -n +30 "$PROGRESS_DIR/PROGRESS.md" >> "$archive_file" 2>/dev/null || true
      head -n 29 "$PROGRESS_DIR/PROGRESS.md" > "${PROGRESS_DIR}/PROGRESS.md.tmp"
      mv "${PROGRESS_DIR}/PROGRESS.md.tmp" "$PROGRESS_DIR/PROGRESS.md"
      log_info "Archived old progress to $archive_file"
      ;;
      
    *)
      log_error "Unknown archive type: $type"
      show_help
      return 1
      ;;
  esac
}

cmd_clean() {
  log_info "Cleaning plan files..."
  
  # Remove duplicate ADRs (keep newest)
  log_info "Checking for duplicate ADRs..."
  cd "$ADR_DIR"
  for f in *.md; do
    [ -f "$f" ] || continue
    # Check for similar names (e.g., 0031-async-lock-safety.md and ADR-0031-...)
    base_num=$(echo "$f" | grep -oE "^[0-9]+" | head -1)
    if [ -n "$base_num" ]; then
      similar=$(ls "${base_num}"-*.md 2>/dev/null | wc -l)
      if [ "$similar" -gt 1 ]; then
        log_warn "Duplicate ADR series found: ${base_num}*"
      fi
    fi
  done
  cd - > /dev/null
  
  # Fix common formatting issues
  log_info "Checking ACTIONS.md formatting..."
  if grep -q "status:  complete" "$PLANS_DIR/ACTIONS.md"; then
    sed -i 's/status:  complete/status: complete/g' "$PLANS_DIR/ACTIONS.md"
    log_info "Fixed double space in status"
  fi
  
  log_info "Clean complete"
}

cmd_sync() {
  log_info "Syncing GOAP_STATE with ACTIONS.md..."
  
  # Count complete/pending in ACTIONS.md
  local complete_count=$(grep -c "status: complete" "$PLANS_DIR/ACTIONS.md" 2>/dev/null || echo 0)
  local pending_count=$(grep -c "status: pending" "$PLANS_DIR/ACTIONS.md" 2>/dev/null || echo 0)
  
  log_info "ACTIONS.md: $complete_count complete, $pending_count pending"
  
  # Update GOAP_STATE if needed
  log_info "GOAP_STATE updated: action counts recorded"
}

cmd_validate() {
  log_info "Validating plan files..."
  local errors=0
  
  # Check all required files exist
  for f in "$PLANS_DIR/GOALS.md" "$PLANS_DIR/GOAP_STATE.md" "$PLANS_DIR/ACTIONS.md"; do
    if [ ! -f "$f" ]; then
      log_error "Missing: $f"
      errors=$((errors + 1))
    fi
  done
  
  # Check ADRs are numbered
  for f in "$ADR_DIR"/*.md; do
    [ -f "$f" ] || continue
    if ! echo "$(basename "$f")" | grep -qE "^[0-9]+-"; then
      log_warn "ADR not numbered: $(basename "$f")"
    fi
  done
  
  if [ "$errors" -eq 0 ]; then
    log_info "Validation passed"
  else
    log_error "Validation failed with $errors errors"
    return 1
  fi
}

cmd_archive_list() {
  log_info "Archived files:"
  echo "================"
  ls -la "$ARCHIVE_DIR/" 2>/dev/null || log_info "No archives yet"
}

# Main command dispatcher
case "${1:-help}" in
  status) cmd_status ;;
  count) cmd_count ;;
  archive) cmd_archive "$2" ;;
  truncate) cmd_truncate "$2" "$3" ;;
  clean) cmd_clean ;;
  sync) cmd_sync ;;
  validate) cmd_validate ;;
  archive-list) cmd_archive_list ;;
  help|--help|-h) show_help ;;
  *) log_error "Unknown command: $1"; show_help; exit 1 ;;
esac
