# FOC Test Visualization System

Comprehensive FOC telemetry collection and visualization for Renode E2E tests.

## Overview

The test visualization system provides:
- **Real-time data collection** from Renode mock peripherals during E2E tests
- **FOC control loop visualization** (position, velocity, currents, PWM)
- **Motion profile analysis** (tracking error, overshoot, settling time)
- **Adaptive control monitoring** (load estimation, coolStep, dcStep)
- **Health monitoring trends** (health score, temperature)
- **PDF report generation** with comprehensive plots and metrics

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Robot Framework Test                       │
│  (example_motion_test_with_viz.robot)                   │
└─────────────────┬───────────────────────────────────────┘
                  │
                  │ uses keywords from
                  ▼
┌─────────────────────────────────────────────────────────┐
│       Test Visualization Keywords                       │
│   (test_visualization_keywords.robot)                   │
│  - Start/Stop Data Collection                           │
│  - Record FOC Snapshots                                 │
│  - Generate Reports                                     │
└─────────────────┬───────────────────────────────────────┘
                  │
         ┌────────┴────────┐
         ▼                 ▼
┌──────────────────┐  ┌──────────────────────┐
│ TestDataCollector│  │ FocTestReportGenerator│
│   (Python)       │  │      (Python)        │
└────────┬─────────┘  └──────────┬───────────┘
         │                       │
         │ saves JSON/CSV        │ generates PDF
         ▼                       ▼
┌─────────────────────────────────────────────────────────┐
│                  test_results/                          │
│  - motion_test.json          (raw data)                 │
│  - motion_test.csv           (for analyze.py)           │
│  - motion_test_full.csv      (all fields)               │
│  - motion_test_report.pdf    (FOC plots)                │
│  - test_suite_summary.pdf    (all tests)                │
└─────────────────────────────────────────────────────────┘
```

## Data Collection

### FocSnapshot Structure

Each snapshot captures a single FOC control loop iteration:

```python
@dataclass
class FocSnapshot:
    timestamp: float          # Relative time (s)
    position: float           # Encoder position (rad)
    velocity: float           # Calculated velocity (rad/s)
    target_position: float    # Target from motion planner (rad)
    target_velocity: float    # Target velocity (rad/s)
    i_q: float                # Q-axis current (A)
    i_d: float                # D-axis current (A)
    load_estimate: float      # Load estimation (Nm)
    pwm_duty_a: float         # Phase A PWM duty (0-1)
    pwm_duty_b: float         # Phase B PWM duty (0-1)
    pwm_duty_c: float         # Phase C PWM duty (0-1)
    temperature: float        # Motor temperature (°C)
    health_score: float       # Health monitor score (0-100)
```

### Data Sources

- **Encoder:** AS5047P mock peripheral (position, velocity)
- **ADC:** Current sense mock (I_q, I_d)
- **Motor Simulator:** PWM duties, temperature, mechanical model
- **Firmware State:** Motion planner targets, load estimates, health score

## Usage

### 1. Quick Start - Run All Tests with Visualization

```bash
./run_tests_with_visualization.sh
```

This will:
1. Build firmware
2. Run all E2E tests with data collection
3. Generate individual PDF reports for each test
4. Generate test suite summary report
5. Display results location

### 2. Write a Test with Visualization

```robot
*** Test Cases ***
My Motion Test With Visualization
    [Documentation]    Test motion with FOC data collection
    
    # Start collecting data
    Start Test Data Collection    my_motion_test
    
    # Run your test
    Send iRPC Command    Enable    node_id=0x10
    Send SetTargetV2 Command    ${1.57}    ${2.0}    ${5.0}    profile=Trapezoidal
    
    # Record FOC snapshots (500ms at 10kHz)
    Record Multiple FOC Snapshots    duration_ms=500    sample_rate_hz=10000
    
    # Stop collecting data
    Stop Test Data Collection
    
    # Generate report
    Generate Test Report    my_motion_test
```

### 3. Manual Data Collection (Python)

```python
from test_data_collector import TestDataCollector

# Create collector
collector = TestDataCollector("my_test")

# Add snapshots during test
for i in range(1000):
    collector.add_from_peripherals(
        encoder_position=position,
        encoder_velocity=velocity,
        adc_i_q=current_q,
        adc_i_d=current_d,
        motor_pwm_a=duty_a,
        motor_pwm_b=duty_b,
        motor_pwm_c=duty_c,
        target_position=target_pos,
        target_velocity=target_vel,
        load_estimate=load,
    )

# Save data
collector.save_json("test_results/my_test.json")
collector.save_pandas_csv("test_results/my_test.csv")
```

### 4. Generate Reports (CLI)

```bash
# Single test report
python3 renode/tests/test_report_generator.py \
    --input test_results/motion_test.json \
    --output test_results/motion_test_report.pdf

# Test suite summary
python3 renode/tests/test_report_generator.py \
    --input test_results/ \
    --suite-summary \
    --output test_results/suite_summary.pdf
```

## Generated Reports

### Individual Test Report (5 pages)

**Page 1: Metadata & Statistics**
- Test name, platform, firmware version, start time
- Sample count, duration
- Performance summary (position range, velocities, currents)

**Page 2: Motion Tracking**
- Position vs time (target vs actual)
- Velocity vs time (target vs actual)
- Position tracking error with statistics (RMS, max, mean)

**Page 3: FOC Control**
- d-q axis currents (I_q for torque, I_d for flux)
- Current statistics (peak, RMS)
- 3-phase PWM duty cycles

**Page 4: Adaptive Control & Diagnostics**
- Load estimation vs time
- Motor temperature
- Health score trend

**Page 5: Phase Diagram**
- Position-velocity phase plot (colored by time)
- Target trajectory overlay

### Test Suite Summary Report

- List of all tests with metadata
- Sample counts and durations
- Status overview

## Integration with Existing Tools

### Compatibility with `analyze.py`

The collected data is compatible with `tools/foc_tools/analyze.py`:

```bash
# Use existing analysis tools on test data
python3 tools/foc_tools/analyze.py \
    --input test_results/motion_test.csv \
    --output test_results/analysis_report.pdf
```

This provides:
- Step response analysis (rise time, overshoot, settling time)
- Frequency response
- Load hysteresis loops

### Real-time Monitoring

For live hardware testing, use `monitor.py`:

```bash
# Real-time FOC monitoring on hardware
python3 -m tools.foc_tools.monitor \
    --interface can0 \
    --node-id 0x0010 \
    --record \
    --output telemetry.csv
```

## File Formats

### JSON Format (`.json`)

Full structured data with metadata and statistics:

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
      ...
    }
  ],
  "statistics": {
    "sample_count": 5000,
    "duration_s": 0.5,
    "position": {"mean": 0.785, "std": 0.453, ...},
    ...
  }
}
```

### CSV Format (`.csv`)

Pandas-compatible format for `analyze.py`:

```csv
time,position,velocity,load,temperature
0.0001,0.0,0.0,0.0,25.0
0.0002,0.001,0.01,0.005,25.0
...
```

### Full CSV Format (`_full.csv`)

All fields from `FocSnapshot`:

```csv
timestamp,position,velocity,target_position,target_velocity,i_q,i_d,load_estimate,pwm_duty_a,pwm_duty_b,pwm_duty_c,temperature,health_score
0.0001,0.0,0.0,1.57,0.0,0.0,0.0,0.0,0.5,0.5,0.5,25.0,100.0
...
```

## Performance Considerations

### Data Collection Overhead

- **Sample rate:** Up to 10 kHz in Renode
- **Memory:** ~200 bytes per snapshot
- **1 second at 10 kHz:** ~2 MB RAM, ~20 MB JSON file

### Recommendations

- For long tests (> 5s), reduce sample rate to 1 kHz
- Use `Record Multiple FOC Snapshots` with appropriate duration
- Collect data only during critical test phases
- Generate reports after test completion (not during)

## Troubleshooting

### No Data Collected

**Symptoms:** Empty JSON files, no PDF reports generated

**Causes:**
- Forgot to call `Start Test Data Collection`
- `Record FOC Snapshots` not called
- Mock peripherals not returning data

**Fix:**
```robot
# Ensure proper lifecycle
Start Test Data Collection    test_name
# ... run test ...
Record Multiple FOC Snapshots    duration_ms=500    sample_rate_hz=10000
Stop Test Data Collection
```

### Missing Dependencies

**Symptoms:** `ModuleNotFoundError: No module named 'matplotlib'`

**Fix:**
```bash
pip3 install matplotlib pandas numpy scipy
```

Or use `run_tests_with_visualization.sh` which auto-installs.

### Report Generation Fails

**Symptoms:** `FileNotFoundError` or empty plots

**Causes:**
- JSON file missing or corrupted
- Insufficient samples (< 2)

**Fix:**
- Verify JSON file exists and has `"samples"` array
- Ensure test collected data before generating report
- Check test logs for data collection errors

### Renode Peripheral Integration

**Current Status:** Mock peripheral read keywords are placeholders

**TODO:** Implement actual Renode Python peripheral API calls

```robot
Get Encoder Position
    [Documentation]    Read encoder position from AS5047P mock
    # Current: Returns placeholder 0.0
    # TODO: Call Renode monitor command to read peripheral state
    ${pos} =    Execute Command    encoder GetPosition
    RETURN    ${pos}
```

## Examples

### Example 1: Motion Profile Test

Test trapezoidal motion from 0 to π/2 rad:

```robot
Test Trapezoidal Profile
    Start Test Data Collection    trap_profile
    
    Send iRPC Command    Enable    node_id=0x10
    Send SetTargetV2 Command    ${1.57}    ${2.0}    ${5.0}    profile=Trapezoidal
    
    Record Multiple FOC Snapshots    duration_ms=500    sample_rate_hz=10000
    
    Stop Test Data Collection
    Generate Test Report    trap_profile
```

**Expected Output:**
- `test_results/trap_profile.json` (raw data)
- `test_results/trap_profile.csv` (for analyze.py)
- `test_results/trap_profile_report.pdf` (5-page PDF with plots)

### Example 2: Adaptive Control Load Step

Test coolStep response to load disturbance:

```robot
Test CoolStep Response
    Start Test Data Collection    coolstep_load_step
    
    Send iRPC Command    Enable    node_id=0x10
    Send ConfigureAdaptive Command    enable_coolstep=${True}
    
    # Baseline (no load)
    Send SetTargetV2 Command    ${1.0}    ${2.0}    ${5.0}
    Record Multiple FOC Snapshots    duration_ms=200    sample_rate_hz=10000
    
    # Apply load
    Set Motor External Load    ${0.3}    # 0.3 Nm
    Record Multiple FOC Snapshots    duration_ms=200    sample_rate_hz=10000
    
    # Remove load
    Set Motor External Load    ${0.0}
    Record Multiple FOC Snapshots    duration_ms=200    sample_rate_hz=10000
    
    Stop Test Data Collection
    Generate Test Report    coolstep_load_step
```

**Expected Output:**
Report shows:
- Load estimate increasing when load applied
- I_q current reduced by coolStep after load is steady
- Current returning to baseline when load removed

### Example 3: High-Speed Motion Stress Test

Test FOC performance at 10 rad/s:

```robot
Test High Speed Motion
    Start Test Data Collection    high_speed_stress
    
    Send iRPC Command    Enable    node_id=0x10
    Send SetTargetV2 Command    ${6.28}    ${10.0}    ${50.0}    profile=SCurve
    
    Record Multiple FOC Snapshots    duration_ms=1000    sample_rate_hz=10000
    
    Stop Test Data Collection
    Generate Test Report    high_speed_stress
```

**Expected Output:**
Report shows:
- Position tracking error during acceleration
- Peak currents during acceleration phase
- PWM duty cycle saturation (if any)
- Temperature rise (if simulated)

## API Reference

### Robot Framework Keywords

#### `Start Test Data Collection`
```robot
Start Test Data Collection    ${test_name}
```
Initialize data collector for test case.

#### `Stop Test Data Collection`
```robot
Stop Test Data Collection
```
Stop collecting and save data files (JSON, CSV).

#### `Record FOC Snapshot`
```robot
Record FOC Snapshot    
    ${encoder_pos}    ${encoder_vel}    ${i_q}    ${i_d}
    ${pwm_a}    ${pwm_b}    ${pwm_c}
    ${target_pos}=0.0    ${target_vel}=0.0
    ${load}=0.0    ${temp}=25.0    ${health}=100.0
```
Record single FOC snapshot with explicit values.

#### `Record Multiple FOC Snapshots`
```robot
Record Multiple FOC Snapshots    
    duration_ms=${500}    
    sample_rate_hz=${10000}
```
Record multiple snapshots from Renode peripherals.

#### `Generate Test Report`
```robot
Generate Test Report    ${test_name}
```
Generate PDF report from collected data.

#### `Generate Suite Summary Report`
```robot
Generate Suite Summary Report
```
Generate summary PDF for all tests in suite.

### Python API

#### `TestDataCollector`

```python
collector = TestDataCollector(test_name: str)
collector.add_snapshot(snapshot: FocSnapshot)
collector.add_from_peripherals(encoder_position, encoder_velocity, ...)
collector.save_json(filepath: str)
collector.save_csv(filepath: str)
collector.save_pandas_csv(filepath: str)
collector.get_statistics() -> dict
```

#### `FocTestReportGenerator`

```python
generator = FocTestReportGenerator(data_file: str)
generator.generate_pdf(output_file: str)
```

#### `MultiTestCollector`

```python
collector = MultiTestCollector(output_dir: str)
test1 = collector.start_test(test_name: str)
test1.add_snapshot(...)
collector.finish_test()
collector.generate_summary() -> dict
collector.save_summary(filename: str)
```

## Future Enhancements

### Planned Features

1. **Live Plotting During Tests**
   - Real-time matplotlib window showing FOC signals
   - Animated phase diagrams

2. **Advanced Analysis**
   - FFT/frequency domain analysis
   - Control loop stability margins (gain/phase margin)
   - Power consumption metrics

3. **Comparative Reports**
   - Compare multiple test runs
   - Regression detection
   - Performance deltas

4. **Export Formats**
   - HDF5 for large datasets
   - MATLAB `.mat` files
   - CSV with headers for external tools

5. **Interactive Dashboards**
   - Web-based HTML reports with Plotly
   - Zoom/pan on time series
   - Selectable signals

## References

- **Motion Planning:** `docs/IRPC_V2_PROTOCOL.md`
- **Adaptive Control:** `docs/IRPC_V2_ADAPTIVE.md`
- **Mock Peripherals:** `renode/PERIPHERALS_README.md`
- **Real-time Monitoring:** `tools/foc_tools/monitor.py`
- **Post-processing:** `tools/foc_tools/analyze.py`

---

**Questions?** Check the example test: `renode/tests/example_motion_test_with_viz.robot`

