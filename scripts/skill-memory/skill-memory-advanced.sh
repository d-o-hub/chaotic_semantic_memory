#!/bin/bash
#
# skill-memory-advanced.sh - Encryption and rate limiting for skill-memory
#
# Usage: source scripts/skill-memory/skill-memory.sh
#        source scripts/skill-memory/skill-memory-advanced.sh
#
# Provides encryption and rate limiting features
#

# Encryption settings
: "${CSM_ENCRYPTION_ENABLED:=false}"
: "${CSM_ENCRYPTION_KEY_FILE:=.agents/csm-memory/.key}"

# Rate limiting settings
: "${CSM_RATE_LIMIT_ENABLED:=false}"
: "${CSM_RATE_LIMIT_OPS_PER_MINUTE:=60}"
: "${CSM_RATE_LIMIT_DIR:=.agents/csm-memory/rate-limits}"

# ============================================================================
# ENCRYPTION
# ============================================================================

# Check if encryption is available
_encryption_available() {
    command -v openssl >/dev/null 2>&1
}

# Generate encryption key
# Usage: skill_encryption_generate_key [--force]
skill_encryption_generate_key() {
    local force=false
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --force) force=true; shift ;;
            *) shift ;;
        esac
    done
    
    if [[ -f "$CSM_ENCRYPTION_KEY_FILE" && "$force" != "true" ]]; then
        echo "Key already exists at: $CSM_ENCRYPTION_KEY_FILE"
        echo "Use --force to regenerate"
        return 1
    fi
    
    if ! _encryption_available; then
        echo "Error: openssl not found for encryption"
        return 1
    fi
    
    # Generate random key
    mkdir -p "$(dirname "$CSM_ENCRYPTION_KEY_FILE")"
    openssl rand -base64 32 > "$CSM_ENCRYPTION_KEY_FILE"
    chmod 600 "$CSM_ENCRYPTION_KEY_FILE"
    
    echo "Encryption key generated: $CSM_ENCRYPTION_KEY_FILE"
    echo "IMPORTANT: Keep this key safe - without it, encrypted data cannot be recovered!"
}

# Encrypt data
# Usage: _encrypt_data "plaintext"
_encrypt_data() {
    local plaintext="$1"
    
    if [[ "$CSM_ENCRYPTION_ENABLED" != "true" ]]; then
        echo "$plaintext"
        return 0
    fi
    
    if [[ ! -f "$CSM_ENCRYPTION_KEY_FILE" ]]; then
        _log_warn "Encryption key not found, storing plaintext"
        echo "$plaintext"
        return 0
    fi
    
    local key
    key=$(cat "$CSM_ENCRYPTION_KEY_FILE")
    
    echo "$plaintext" | openssl enc -aes-256-cbc -base64 -A -salt -pass pass:"$key" 2>/dev/null
}

# Decrypt data
# Usage: _decrypt_data "encrypted_base64"
_decrypt_data() {
    local encrypted="$1"
    
    if [[ "$CSM_ENCRYPTION_ENABLED" != "true" ]]; then
        echo "$encrypted"
        return 0
    fi
    
    if [[ ! -f "$CSM_ENCRYPTION_KEY_FILE" ]]; then
        echo "$encrypted"
        return 0
    fi
    
    local key
    key=$(cat "$CSM_ENCRYPTION_KEY_FILE")
    
    echo "$encrypted" | openssl enc -aes-256-cbc -d -base64 -A -pass pass:"$key" 2>/dev/null || echo "$encrypted"
}

# Enable encryption
# Usage: skill_encryption_enable ["key_file"]
skill_encryption_enable() {
    local key_file="${1:-$CSM_ENCRYPTION_KEY_FILE}"
    
    if [[ ! -f "$key_file" ]]; then
        echo "Error: Key file not found: $key_file"
        echo "Run: skill_encryption_generate_key"
        return 1
    fi
    
    CSM_ENCRYPTION_KEY_FILE="$key_file"
    CSM_ENCRYPTION_ENABLED=true
    
    echo "Encryption enabled with key: $key_file"
}

# Disable encryption
skill_encryption_disable() {
    CSM_ENCRYPTION_ENABLED=false
    echo "Encryption disabled"
}

# Get encryption status
skill_encryption_status() {
    echo "Encryption Status:"
    echo "  Enabled:  $CSM_ENCRYPTION_ENABLED"
    echo "  Key:      ${CSM_ENCRYPTION_KEY_FILE:-none}"
    echo "  Available: $(_encryption_available && echo 'yes' || echo 'no (openssl not found)')"
}

# ============================================================================
# RATE LIMITING
# ============================================================================

# Initialize rate limiting
_rate_limit_init() {
    mkdir -p "$CSM_RATE_LIMIT_DIR"
}

# Check if rate limit exceeded
# Usage: _check_rate_limit "skill_name"
_check_rate_limit() {
    local skill="$1"
    
    [[ "$CSM_RATE_LIMIT_ENABLED" != "true" ]] && return 0
    
    _rate_limit_init
    
    local limit_file="$CSM_RATE_LIMIT_DIR/${skill}.limit"
    local now
    now=$(date +%s)
    
    # Create or update limit file
    if [[ ! -f "$limit_file" ]]; then
        echo "$now 1" > "$limit_file"
        return 0
    fi
    
    # Read last request time and count
    local last_time count
    last_time=$(cut -d' ' -f1 "$limit_file")
    count=$(cut -d' ' -f2 "$limit_file")
    
    local elapsed=$((now - last_time))
    
    # Reset if more than 1 minute passed
    if [[ $elapsed -gt 60 ]]; then
        echo "$now 1" > "$limit_file"
        return 0
    fi
    
    # Check if limit exceeded
    if [[ $count -ge $CSM_RATE_LIMIT_OPS_PER_MINUTE ]]; then
        _log_error "Rate limit exceeded for $skill: $count ops in last minute (max: $CSM_RATE_LIMIT_OPS_PER_MINUTE)"
        return 1
    fi
    
    # Increment count
    count=$((count + 1))
    echo "$last_time $count" > "$limit_file"
    
    return 0
}

# Get rate limit status
skill_rate_limit_status() {
    echo "Rate Limiting Status:"
    echo "  Enabled:       $CSM_RATE_LIMIT_ENABLED"
    echo "  Max ops/min:    $CSM_RATE_LIMIT_OPS_PER_MINUTE"
    echo "  Storage:        $CSM_RATE_LIMIT_DIR"
    
    if [[ -d "$CSM_RATE_LIMIT_DIR" ]]; then
        echo ""
        echo "  Current usage:"
        for f in "$CSM_RATE_LIMIT_DIR"/*.limit; do
            [[ ! -f "$f" ]] && continue
            local skill
            skill=$(basename "$f" .limit)
            local count
            count=$(cut -d' ' -f2 "$f")
            echo "    $skill: $count ops/min"
        done
    fi
}

# Configure rate limiting
# Usage: skill_rate_limit_config [--enable|--disable] [--max-ops N]
skill_rate_limit_config() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --enable)
                CSM_RATE_LIMIT_ENABLED=true
                _rate_limit_init
                echo "Rate limiting: enabled"
                shift
                ;;
            --disable)
                CSM_RATE_LIMIT_ENABLED=false
                echo "Rate limiting: disabled"
                shift
                ;;
            --max-ops)
                CSM_RATE_LIMIT_OPS_PER_MINUTE=$2
                echo "Rate limiting: max $2 ops/minute"
                shift 2
                ;;
            *)
                echo "Unknown option: $1"
                return 1
                ;;
        esac
    done
}

# Clear rate limits
skill_rate_limit_clear() {
    if [[ -d "$CSM_RATE_LIMIT_DIR" ]]; then
        rm -f "$CSM_RATE_LIMIT_DIR"/*.limit
        echo "Rate limits cleared"
    else
        echo "No rate limits to clear"
    fi
}

# ============================================================================
# Export functions
# ============================================================================

if [[ "${BASH_SOURCE[0]}" != "${0}" ]]; then
    export -f skill_encryption_generate_key
    export -f skill_encryption_enable
    export -f skill_encryption_disable
    export -f skill_encryption_status
    export -f skill_rate_limit_status
    export -f skill_rate_limit_config
    export -f skill_rate_limit_clear
    export CSM_ENCRYPTION_ENABLED CSM_ENCRYPTION_KEY_FILE
    export CSM_RATE_LIMIT_ENABLED CSM_RATE_LIMIT_OPS_PER_MINUTE CSM_RATE_LIMIT_DIR
fi
