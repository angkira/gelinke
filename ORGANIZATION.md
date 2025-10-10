# 📁 Project Organization

**Last cleanup:** 2025-10-10

## Quick Reference

| What you need | Where to find it |
|--------------|------------------|
| **Session reports** | `docs/reports/` |
| **Guides & tutorials** | `docs/guides/` |
| **Demo scripts** | `scripts/demos/` |
| **Analysis tools** | `scripts/analysis/` |
| **Test results** | `demo_results/` |
| **MPC code & data** | `mpc/` |
| **Renode tests** | `renode/tests/` |

## Directory Structure

```
joint_firmware/
├── .claude/              # AI assistant configuration
│   └── project_rules.md  # File organization rules
├── docs/
│   ├── reports/          # Session summaries (7 files)
│   └── guides/           # Implementation guides (4 files)
├── scripts/
│   ├── analysis/         # Analysis scripts (4 files)
│   ├── demos/            # Demo scripts (demo_visualization.py)
│   ├── validation/       # Validation scripts (empty)
│   └── build/            # Build utilities (empty)
├── src/                  # Rust firmware source
├── renode/               # Renode simulation & Python tests
├── mpc/                  # MPC implementation & data
├── demo_results/         # Generated outputs (plots, JSON, CSV)
└── [build files]         # Cargo.toml, Dockerfile, etc.
```

## Rules for Contributors

### ✅ DO:
- Put reports in `docs/reports/`
- Put guides in `docs/guides/`
- Put scripts in `scripts/<category>/`
- Put test results in `demo_results/`
- Keep root clean!

### ❌ DON'T:
- Create report files in root
- Create analysis scripts in root
- Leave temporary files around
- Commit `__pycache__` or build artifacts

## Key Files Moved (2025-10-10 cleanup)

**From root → docs/reports/:**
- COMPREHENSIVE_SESSION_REPORT.md
- FOC_VISUALIZATION_COMPLETE.md
- SESSION_COMPLETE_FOC_VISUALIZATION.md
- SESSION_SUMMARY_INPUT_SHAPING.md
- PHASE_1_COMPLETE.md
- PHASE_2_COMPLETE.md
- PHASE_3_COMPLETE.md

**From root → docs/guides/:**
- FOC_VISUALIZATION_README.md
- INPUT_SHAPING_GUIDE.md
- DOCKER_TESTS_README.md
- PROMPT_FOR_VALIDATION_AGENT.md

**From root → scripts/:**
- demo_visualization.py → scripts/demos/
- analyze_tracking_error.py → scripts/analysis/
- compare_trajectories.py → scripts/analysis/
- fix_overshoot.py → scripts/analysis/
- optimize_scurve_controller.py → scripts/analysis/

## For Claude Code

See `.claude/project_rules.md` for detailed file organization rules.

**TL;DR:** Don't create files in root unless they're build configs!

---

*Enforced by Claude Code AI Assistant*


