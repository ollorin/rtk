# RTK Installation (Enhanced Fork)

Quick installation guide for the enhanced RTK fork with Supabase, Nx, and Deno support.

## Prerequisites

macOS with Homebrew (optional but recommended)

## Installation (5 minutes)

### Step 1: Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
```

### Step 2: Clone and Build

```bash
# Clone the enhanced fork
git clone https://github.com/ollorin/rtk.git
cd rtk

# Build release version
cargo build --release

# Install to user bin
mkdir -p ~/.local/bin
cp target/release/rtk ~/.local/bin/
chmod +x ~/.local/bin/rtk
```

### Step 3: Add to PATH

Add to your `~/.zshrc` (or `~/.bashrc`):

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Step 4: Verify Installation

```bash
rtk --version
# Should show: rtk 0.14.0

rtk gain
# Should show: "No tracking data yet"
```

### Step 5: Configure Hook (Recommended)

**For Claude Code users only:**

```bash
# Install hook + configure globally
rtk init -g --auto-patch

# Restart Claude Code after this
```

**What this does:**
- Installs auto-rewrite hook to `~/.claude/hooks/rtk-rewrite.sh`
- Creates minimal `~/.claude/RTK.md` (10 lines)
- Patches `~/.claude/settings.json` (backup created automatically)

## What You Get

### New Commands

**Supabase:**
```bash
rtk supabase start    # Compact startup (150 lines → 5 lines)
rtk supabase status   # Clean table format
rtk supabase db push  # Migration summary only
```

**Nx:**
```bash
rtk nx test api            # No task graph spam
rtk nx build player-web    # Progress + completion only
rtk nx affected:test       # Clean project list
```

**Deno:**
```bash
rtk deno test       # 383 steps → 1 summary line
rtk deno lint       # Errors/warnings only
rtk deno check      # Type errors only
```

### Existing Commands (Already Included)

```bash
rtk git status      # Compact git output
rtk gh pr list      # Clean PR listing
rtk docker ps       # Simplified container list
rtk ls              # Token-optimized directory tree
rtk grep "pattern"  # Grouped search results
```

## Hook Behavior

If you installed the hook, commands are **automatically rewritten**:

```bash
# You type:              # RTK executes:
deno test        →       rtk deno test
npx nx test api  →       rtk nx test api
supabase start   →       rtk supabase start
git status       →       rtk git status
```

**Transparent** - you won't see the rewrite, just optimized output!

## Verify It's Working

```bash
# Check token savings
rtk gain

# See which commands you're using
rtk gain --history

# Find missed opportunities
rtk discover --all
```

## Token Savings Examples

**Before RTK:**
```
supabase start: 150 lines (container logs, migrations, etc.)
deno test: 383 test steps with verbose assertions
npx nx test api: Full test output + Nx task graph
```

**With RTK:**
```
supabase start: ✓ Started (54321) | Keys: anon_*** service_***
deno test: ok ✓ 102 passed (383 steps) | 0 failed
npx nx test api: ✓ 23 passed | ⚠️ 2 skipped | ✗ 1 failed
```

**Result**: 70-90% fewer tokens per session

## Troubleshooting

### "rtk: command not found"

```bash
# Check PATH
echo $PATH | grep -o '[^:]*\.local[^:]*'

# If missing, add to shell config
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### "cargo: command not found"

Rust not installed. Run Step 1 again.

### Hook not working

```bash
# Verify hook is installed
rtk init --show

# Should show:
# ✅ Hook: /Users/you/.claude/hooks/rtk-rewrite.sh (executable)
# ✅ settings.json: RTK hook configured

# If not, reinstall:
rtk init -g --auto-patch
```

Then **restart Claude Code**.

### Verify correct RTK (not Type Kit)

```bash
rtk gain
# Should show token stats, NOT "command not found"
```

If `rtk gain` fails, you installed the wrong package. See main README.

## Uninstall

```bash
# Remove hook
rtk init -g --uninstall

# Remove binary
rm ~/.local/bin/rtk

# Remove Rust (optional)
rustup self uninstall
```

## Support

- **Issues**: https://github.com/ollorin/rtk/issues
- **Upstream**: https://github.com/rtk-ai/rtk
- **PR with these features**: https://github.com/rtk-ai/rtk/pull/91

---

**tl;dr**: Install Rust → Clone → `cargo build --release` → Copy to `~/.local/bin` → `rtk init -g` → Restart Claude Code → Profit 🎉
