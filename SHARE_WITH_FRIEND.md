# Install RTK (Enhanced) - For Mac Users

Hey! Here's how to install the enhanced RTK that saves ~70% of tokens in Claude Code.

## What You'll Get

- **Supabase CLI**: `rtk supabase start` (150 lines → 5 lines)
- **Nx commands**: `rtk nx test api` (no task graph spam)
- **Deno tools**: `rtk deno test` (383 steps → 1 summary line)
- **Plus 30+ other commands**: git, docker, gh, pnpm, vitest, etc.

Result: **60-90% fewer tokens** per session

---

## Installation (Copy & Paste This)

Open Terminal and run:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
  source ~/.cargo/env && \
  git clone https://github.com/ollorin/rtk.git && \
  cd rtk && cargo build --release && \
  mkdir -p ~/.local/bin && cp target/release/rtk ~/.local/bin/ && \
  echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && \
  source ~/.zshrc && \
  rtk --version
```

**Expected**: Shows `rtk 0.14.0`

---

## Setup Hook (For Claude Code)

If you use Claude Code, run this:

```bash
rtk init -g --auto-patch
```

Then **restart Claude Code**.

---

## Test It

Try these commands:

```bash
# Check token savings
rtk gain

# See what commands you can optimize
rtk discover --all

# Test a command
rtk git status
```

---

## How It Works

Once installed, RTK automatically rewrites your commands:

```
You type:           RTK executes:
deno test     →     rtk deno test
git status    →     rtk git status
supabase start →    rtk supabase start
```

**You won't see the rewrite** - just cleaner output!

---

## Troubleshooting

**"rtk: command not found"**
```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

**Verify correct RTK (not Type Kit):**
```bash
rtk gain  # Should show token stats, NOT error
```

---

## Full Docs

- Installation guide: https://github.com/ollorin/rtk/blob/feat/supabase-nx-deno-support/INSTALL_FORK.md
- Main README: https://github.com/ollorin/rtk

---

**Questions?** Hit me up! 🚀
