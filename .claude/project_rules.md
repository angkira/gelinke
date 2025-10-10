# Project Rules for Claude Code

## 📁 File Organization Rules

### ❌ DO NOT create these files in project root:

**Reports & Documentation:**
- Session reports → `docs/reports/`
- Guides & tutorials → `docs/guides/`
- Technical documentation → `docs/`
- README files (except main README.md) → `docs/`

**Scripts & Tools:**
- Python analysis scripts → `scripts/analysis/`
- Demo/test scripts → `scripts/demos/`
- Validation scripts → `scripts/validation/`
- Build/deployment scripts → `scripts/build/`

**Test Data & Results:**
- Test outputs → `demo_results/` or `tests/results/`
- Generated plots → `demo_results/plots/`
- JSON/CSV data → `demo_results/`
- Temporary files → `.tmp/` or `target/`

**Configuration:**
- Test configs → `tests/configs/`
- Motor models → `mpc/models/` or `configs/`

### ✅ Allowed in project root:

- `Cargo.toml`, `Cargo.lock` - Rust package config
- `build.rs` - Build script
- `Dockerfile.*` - Container definitions
- `docker-compose.yml` - Docker orchestration
- `.gitignore`, `.dockerignore` - VCS config
- `README.md` - Main project readme
- Shell scripts for CI/CD: `run_tests.sh`, `build.sh`, etc.

### 📂 Directory Structure:

```
joint_firmware/
├── .cargo/           # Rust toolchain config
├── .github/          # GitHub Actions
├── docs/
│   ├── reports/      # Session summaries, completion reports
│   ├── guides/       # User guides, tutorials
│   └── api/          # API documentation
├── scripts/
│   ├── analysis/     # Analysis & plotting scripts
│   ├── demos/        # Demo scripts
│   ├── validation/   # Validation scripts
│   └── build/        # Build utilities
├── src/              # Rust source code
├── tests/            # Rust integration tests
├── renode/           # Renode simulation
│   └── tests/        # Python test scripts (keep here)
├── mpc/              # MPC-specific code & data
├── demo_results/     # Generated test results
└── notebooks/        # Jupyter notebooks
```

## 🎯 When Creating New Files:

### Before creating a file, ask:
1. **Is it documentation?** → `docs/reports/` or `docs/guides/`
2. **Is it a script?** → `scripts/<category>/`
3. **Is it test data?** → `demo_results/` or `tests/results/`
4. **Is it temporary?** → `.tmp/` or use `tempfile` in script

### Examples:

```
❌ BAD:  PHASE_4_COMPLETE.md (in root)
✅ GOOD: docs/reports/PHASE_4_COMPLETE.md

❌ BAD:  analyze_mpc_performance.py (in root)
✅ GOOD: scripts/analysis/analyze_mpc_performance.py

❌ BAD:  validation_results.json (in root)
✅ GOOD: demo_results/validation_results.json

❌ BAD:  MPC_IMPLEMENTATION_GUIDE.md (in root)
✅ GOOD: docs/guides/MPC_IMPLEMENTATION_GUIDE.md
```

## 🧹 Cleanup Rules:

### Auto-cleanup (should be in .gitignore):
- `__pycache__/` - Python cache
- `*.pyc` - Python bytecode
- `target/` - Rust build artifacts
- `.tmp/` - Temporary files
- `*.log` - Log files (unless explicitly tracked)

### Move existing files:
If you find misplaced files in root:
```bash
# Reports
mv *_COMPLETE.md SESSION_*.md docs/reports/

# Guides  
mv *_GUIDE.md *_README.md docs/guides/

# Scripts
mv analyze_*.py compare_*.py fix_*.py scripts/analysis/
mv demo_*.py scripts/demos/

# Results
mv *.png *.json demo_results/
```

## 💡 Best Practices:

1. **Ask before creating in root** - "Should this go in scripts/ or docs/?"
2. **Use descriptive paths** - `scripts/analysis/mpc_tracking.py` not `analyze.py`
3. **Group related files** - Keep all MPC stuff in `mpc/`
4. **Clean up after yourself** - Remove temp files after validation
5. **Check .gitignore** - Don't commit build artifacts

## 🚨 Exceptions:

These scripts CAN stay in root (for convenience):
- `run_tests.sh` - Main test runner
- `build.sh` - Build script
- `deploy.sh` - Deployment script
- `install-*.sh` - Installation scripts

But prefer `scripts/` for specialized scripts!

---

*Last updated: 2025-10-10*
*Enforced by: Claude Code AI Assistant*


