# FOC Test Visualization System

**Comprehensive FOC telemetry collection and visualization for Renode E2E tests**

## 🎯 Overview

The FOC Test Visualization System provides **real-time data collection** from Renode mock peripherals during E2E tests and **automatic generation of comprehensive PDF reports** with FOC control loop analysis.

### Key Features

✅ **Data Collection from Renode Mock Peripherals**
- AS5047P encoder (position, velocity)
- 3-phase current sense ADC (I_q, I_d)
- Motor simulator (PWM duties, temperature, load)
- Firmware state (motion planner targets, health score)

✅ **Comprehensive FOC Visualization**
- Motion tracking (position, velocity, target vs actual)
- Tracking error analysis (RMS, max, mean errors)
- d-q axis currents and statistics
- 3-phase PWM duty cycles
- Phase diagrams (position-velocity)

✅ **Adaptive Control Monitoring**
- Load estimation trends
- coolStep current reduction
- Temperature monitoring
- Health score tracking

✅ **Automated Report Generation**
- Individual test PDFs (5 pages with plots + metrics)
- Test suite summary PDFs
- CSV export compatible with existing `analyze.py` tools

## 🚀 Quick Start

### 1. Install Dependencies

```bash
pip3 install -r requirements-viz.txt
```

### 2. Run Demo (See Example Reports)

```bash
./demo_visualization.py
```

This generates:
- `demo_results/*.json` - Raw telemetry data
- `demo_results/*.csv` - Pandas-compatible CSV
- `demo_results/*_report.pdf` - 5-page PDF reports with FOC plots
- `demo_results/demo_suite_summary.pdf` - Summary of all tests

**Open the PDFs to see the visualization capabilities!**

### 3. Run E2E Tests with Visualization

```bash
./run_tests_with_visualization.sh
```

This will:
1. Build firmware
2. Run all E2E tests with data collection
3. Generate individual PDF reports for each test
4. Generate test suite summary report

Results will be in `test_results/` directory.

## 📊 What's Included

### Python Modules

| File | Purpose |
|------|---------|
| `renode/tests/test_data_collector.py` | Collects FOC telemetry data from tests |
| `renode/tests/test_report_generator.py` | Generates PDF reports with FOC plots |
| `renode/tests/test_visualization_keywords.robot` | Robot Framework keywords |
| `demo_visualization.py` | Demo script with example data |

### Robot Framework Integration

| Keyword | Description |
|---------|-------------|
| `Start Test Data Collection` | Initialize data collector |
| `Record FOC Snapshot` | Record single FOC loop iteration |
| `Record Multiple FOC Snapshots` | Record N samples at specified rate |
| `Stop Test Data Collection` | Save collected data |
| `Generate Test Report` | Generate PDF report |
| `Generate Suite Summary Report` | Generate summary for all tests |

### Example Test

```robot
*** Test Cases ***
Test Motion Profile With Visualization
    [Documentation]    Test with full FOC data collection
    
    # Start collecting data
    Start Test Data Collection    motion_test
    
    # Run your test
    Send iRPC Command    Enable    node_id=0x10
    Send SetTargetV2 Command    ${1.57}    ${2.0}    ${5.0}    profile=Trapezoidal
    
    # Record FOC snapshots (500ms at 10kHz = 5000 samples)
    Record Multiple FOC Snapshots    duration_ms=500    sample_rate_hz=10000
    
    # Verify
    ${final_pos} =    Get Encoder Position
    Should Be Close    ${final_pos}    ${1.57}    tolerance=0.01
    
    # Stop collecting and generate report
    Stop Test Data Collection
    Generate Test Report    motion_test
```

## 📈 Generated Reports

### Individual Test Report (5 Pages)

**Page 1: Metadata & Statistics**
```
┌─────────────────────────────────────┐
│  Test: motion_test                  │
│  Platform: Renode STM32G431CB       │
│  Firmware: iRPC v2.0                │
│  Samples: 5000                      │
│  Duration: 0.500 s                  │
│                                     │
│  Performance Summary:               │
│    Position Range: [0.0, 1.57] rad  │
│    Position Std Dev: 0.453 rad      │
│    Max Velocity: 2.0 rad/s          │
│    Peak Current: 1.2 A              │
└─────────────────────────────────────┘
```

**Page 2: Motion Tracking**
- Position vs time (target vs actual, dual axis)
- Velocity vs time (target vs actual)
- Position tracking error with ±1° tolerance band
- Statistics: RMS error, max error, mean error

**Page 3: FOC Control**
- d-q axis currents (I_q for torque, I_d for flux)
- Current statistics (peak I_q, RMS current, magnitude)
- 3-phase PWM duty cycles (0-1 range)

**Page 4: Adaptive Control & Diagnostics**
- Load estimation vs time
- Motor temperature (dual axis)
- Health score (0-100) with warning/critical thresholds
- Color-coded health regions (green/yellow/red)

**Page 5: Phase Diagram**
- Position-velocity phase plot (colored by time)
- Target trajectory overlay
- Shows motion dynamics and stability

### Test Suite Summary

Lists all tests with:
- Test names
- Sample counts
- Durations
- Status overview

## 🔧 Integration with Existing Tools

### Compatibility with `tools/foc_tools/analyze.py`

The collected CSV data is compatible:

```bash
# Use existing analysis tools on test data
python3 tools/foc_tools/analyze.py \
    --input test_results/motion_test.csv \
    --output analysis_report.pdf \
    --analysis step_response
```

This adds:
- Step response analysis (rise time, overshoot, settling time)
- Frequency response
- Load hysteresis loops

### Real-time Monitoring (Hardware)

For live hardware testing:

```bash
python3 -m tools.foc_tools.monitor \
    --interface can0 \
    --node-id 0x0010 \
    --record \
    --output telemetry.csv
```

Then visualize with:

```bash
python3 tools/foc_tools/analyze.py \
    --input telemetry.csv \
    --output hardware_report.pdf
```

## 📁 File Formats

### JSON Format

Full structured data with metadata:

```json
{
  "metadata": {
    "test_name": "motion_test",
    "start_time": "2025-10-07 12:34:56",
    "platform": "Renode STM32G431CB",
    "firmware_version": "iRPC v2.0"
  },
  "samples": [
    {
      "timestamp": 0.0001,
      "position": 0.0,
      "velocity": 0.0,
      "target_position": 0.0,
      "target_velocity": 0.0,
      "i_q": 0.0,
      "i_d": 0.0,
      "load_estimate": 0.0,
      "pwm_duty_a": 0.5,
      "pwm_duty_b": 0.5,
      "pwm_duty_c": 0.5,
      "temperature": 25.0,
      "health_score": 100.0
    },
    ...
  ],
  "statistics": {
    "sample_count": 5000,
    "duration_s": 0.5,
    "position": {"mean": 0.785, "std": 0.453, "min": 0.0, "max": 1.57},
    "velocity": {"mean": 1.0, "std": 0.5, "min": 0.0, "max": 2.0},
    "current_q": {"mean": 0.5, "std": 0.2, "peak": 1.2},
    "load_estimate": {"mean": 0.1, "std": 0.05, "max": 0.2}
  }
}
```

### CSV Format (Pandas-compatible)

For use with `analyze.py`:

```csv
time,position,velocity,load,temperature
0.0001,0.0,0.0,0.0,25.0
0.0002,0.001,0.01,0.005,25.0
...
```

## 🎨 Example Visualizations

### Trapezoidal Motion Profile
- Position ramps up linearly (accel phase)
- Position plateau (coast phase, if any)
- Position ramps to target (decel phase)
- Velocity: trapezoid shape
- Current I_q: spikes during accel/decel, low during coast
- Tracking error: minimal during motion, settles to < 1°

### Adaptive Control Load Step
- Load estimate: step from 0 → 0.3 Nm → 0 Nm
- Current I_q: increases with load, then reduces with coolStep
- Position: slight disturbance when load applied
- Health score: degrades slightly under load, recovers

### High-Speed Motion
- Position: rapid increase to 2π rad
- Velocity: peaks at 10 rad/s
- Current I_q: saturates at ±5 A limit
- PWM: hard saturation at 0/1 during high speed
- Temperature: rapid rise
- Health score: degrades with high speed

## 📖 Full Documentation

See `docs/TEST_VISUALIZATION.md` for:
- Detailed architecture
- API reference
- Troubleshooting
- Advanced examples
- Performance considerations

## 🛠️ Development

### Add New Plot

Edit `renode/tests/test_report_generator.py`:

```python
def plot_my_custom_analysis(self, ax: plt.Axes):
    """Plot custom analysis."""
    ax.plot(self.df["timestamp"], self.df["position"], "b-")
    ax.set_xlabel("Time (s)")
    ax.set_ylabel("Position (rad)")
    ax.set_title("My Custom Plot")
    ax.grid(True)
```

Then add to `generate_pdf()`:

```python
# Page 6: Custom analysis
fig, ax = plt.subplots(1, 1, figsize=(11, 8.5))
self.plot_my_custom_analysis(ax)
pdf.savefig(fig, bbox_inches="tight")
plt.close()
```

### Collect Custom Data

Add field to `FocSnapshot` in `test_data_collector.py`:

```python
@dataclass
class FocSnapshot:
    # ... existing fields ...
    custom_metric: float = 0.0  # My custom metric
```

Then collect in test:

```python
collector.add_snapshot(FocSnapshot(
    # ... existing fields ...
    custom_metric=my_value,
))
```

## 📊 Performance

### Data Collection Overhead
- **Sample rate:** Up to 10 kHz in Renode
- **Memory:** ~200 bytes per snapshot
- **1 second at 10 kHz:** ~2 MB RAM, ~20 MB JSON

### Recommendations
- For long tests (> 5s), reduce sample rate to 1 kHz
- Collect data only during critical phases
- Generate reports after test completion (not during)

## 🐛 Troubleshooting

### No Data Collected

❌ **Problem:** Empty JSON files

✅ **Solution:** Ensure proper lifecycle:
```robot
Start Test Data Collection    test_name
# ... run test ...
Record Multiple FOC Snapshots    duration_ms=500    sample_rate_hz=10000
Stop Test Data Collection
```

### Missing Dependencies

❌ **Problem:** `ModuleNotFoundError: No module named 'matplotlib'`

✅ **Solution:**
```bash
pip3 install -r requirements-viz.txt
```

### Report Generation Fails

❌ **Problem:** Empty plots or `FileNotFoundError`

✅ **Solution:**
- Verify JSON file exists and has `"samples"` array
- Ensure test collected data (> 2 samples)
- Check test logs for errors

## 🎯 Next Steps

1. **Run the demo:** `./demo_visualization.py`
2. **Open the PDFs:** See example reports in `demo_results/`
3. **Write your own test:** Use `example_motion_test_with_viz.robot` as template
4. **Run full test suite:** `./run_tests_with_visualization.sh`

## 📚 References

- **Motion Planning:** `docs/IRPC_V2_PROTOCOL.md`
- **Adaptive Control:** `docs/IRPC_V2_ADAPTIVE.md`
- **Mock Peripherals:** `renode/PERIPHERALS_README.md`
- **Full Viz Docs:** `docs/TEST_VISUALIZATION.md`
- **Real-time Monitor:** `tools/foc_tools/monitor.py`
- **Post-processing:** `tools/foc_tools/analyze.py`

---

**🎉 Enjoy comprehensive FOC visualization for your robotic joint tests!**

