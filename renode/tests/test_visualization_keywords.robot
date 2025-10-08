*** Settings ***
Documentation    Robot Framework keywords for test data collection and visualization
Library          test_data_collector.py
Library          test_report_generator.py
Library          Collections

*** Variables ***
${TEST_RESULTS_DIR}    ${CURDIR}/../../test_results
${CURRENT_COLLECTOR}    ${NONE}

*** Keywords ***
Start Test Data Collection
    [Documentation]    Start collecting FOC telemetry for current test
    [Arguments]    ${test_name}
    
    Log    Starting data collection for: ${test_name}    console=yes
    ${collector} =    Evaluate    __import__('test_data_collector').TestDataCollector('${test_name}')
    Set Global Variable    ${CURRENT_COLLECTOR}    ${collector}
    Set Test Variable    ${TEST_NAME}    ${test_name}
    
    Log    Data collector initialized    console=yes

Stop Test Data Collection
    [Documentation]    Stop collecting data and save results
    
    Log    Stopping data collection    console=yes
    
    IF    '${CURRENT_COLLECTOR}' != '${NONE}'
        # Save data files
        ${json_file} =    Set Variable    ${TEST_RESULTS_DIR}/${TEST_NAME}.json
        ${csv_file} =    Set Variable    ${TEST_RESULTS_DIR}/${TEST_NAME}.csv
        ${full_csv_file} =    Set Variable    ${TEST_RESULTS_DIR}/${TEST_NAME}_full.csv
        
        Run Keyword    Create Directory    ${TEST_RESULTS_DIR}
        
        Evaluate    ${CURRENT_COLLECTOR}.save_json('${json_file}')
        Evaluate    ${CURRENT_COLLECTOR}.save_pandas_csv('${csv_file}')
        Evaluate    ${CURRENT_COLLECTOR}.save_csv('${full_csv_file}')
        
        Log    Test data saved to ${TEST_RESULTS_DIR}/${TEST_NAME}.*    console=yes
        
        Set Global Variable    ${CURRENT_COLLECTOR}    ${NONE}
    END

Record FOC Snapshot
    [Documentation]    Record single FOC control loop snapshot from mock peripherals
    [Arguments]    ${encoder_pos}    ${encoder_vel}    ${i_q}    ${i_d}    
    ...            ${pwm_a}    ${pwm_b}    ${pwm_c}
    ...            ${target_pos}=0.0    ${target_vel}=0.0    
    ...            ${load}=0.0    ${temp}=25.0    ${health}=100.0
    
    IF    '${CURRENT_COLLECTOR}' != '${NONE}'
        Evaluate    ${CURRENT_COLLECTOR}.add_from_peripherals(${encoder_pos}, ${encoder_vel}, ${i_q}, ${i_d}, ${pwm_a}, ${pwm_b}, ${pwm_c}, ${target_pos}, ${target_vel}, ${load}, ${temp}, ${health})
    END

Record Multiple FOC Snapshots
    [Documentation]    Record multiple snapshots from Renode simulation
    [Arguments]    ${duration_ms}    ${sample_rate_hz}=10000
    
    ${samples} =    Evaluate    int(${duration_ms} * ${sample_rate_hz} / 1000)
    
    Log    Recording ${samples} FOC snapshots over ${duration_ms} ms    console=yes
    
    FOR    ${i}    IN RANGE    ${samples}
        # Read values from Renode mock peripherals
        ${encoder_pos} =    Get Encoder Position
        ${encoder_vel} =    Get Encoder Velocity
        ${i_q} =    Get Current Q
        ${i_d} =    Get Current D
        ${pwm_a} =    Get PWM Duty A
        ${pwm_b} =    Get PWM Duty B
        ${pwm_c} =    Get PWM Duty C
        ${target_pos} =    Get Target Position
        ${target_vel} =    Get Target Velocity
        ${load} =    Get Load Estimate
        ${temp} =    Get Motor Temperature
        ${health} =    Get Health Score
        
        Record FOC Snapshot    ${encoder_pos}    ${encoder_vel}    ${i_q}    ${i_d}
        ...                    ${pwm_a}    ${pwm_b}    ${pwm_c}
        ...                    ${target_pos}    ${target_vel}
        ...                    ${load}    ${temp}    ${health}
        
        # Advance simulation
        Sleep    ${1.0 / ${sample_rate_hz}}s
    END
    
    Log    Recorded ${samples} snapshots    console=yes

Generate Test Report
    [Documentation]    Generate PDF report from collected test data
    [Arguments]    ${test_name}
    
    ${json_file} =    Set Variable    ${TEST_RESULTS_DIR}/${test_name}.json
    ${pdf_file} =    Set Variable    ${TEST_RESULTS_DIR}/${test_name}_report.pdf
    
    Log    Generating report: ${pdf_file}    console=yes
    
    Evaluate    __import__('test_report_generator').FocTestReportGenerator('${json_file}').generate_pdf('${pdf_file}')
    
    Log    Report generated: ${pdf_file}    console=yes

Generate Suite Summary Report
    [Documentation]    Generate summary report for all tests
    
    ${output_file} =    Set Variable    ${TEST_RESULTS_DIR}/test_suite_summary.pdf
    
    Log    Generating test suite summary...    console=yes
    
    Evaluate    __import__('test_report_generator').generate_test_suite_summary('${TEST_RESULTS_DIR}', '${output_file}')
    
    Log    Suite summary: ${output_file}    console=yes

# Mock peripheral read keywords (to be implemented with actual Renode API)
Get Encoder Position
    [Documentation]    Read encoder position from AS5047P mock
    # TODO: Connect to Renode Python peripheral
    ${pos} =    Evaluate    0.0  # Placeholder
    RETURN    ${pos}

Get Encoder Velocity
    [Documentation]    Read encoder velocity (calculated)
    # TODO: Connect to Renode Python peripheral
    ${vel} =    Evaluate    0.0  # Placeholder
    RETURN    ${vel}

Get Current Q
    [Documentation]    Read Q-axis current from ADC mock
    # TODO: Connect to Renode Python peripheral
    ${iq} =    Evaluate    0.0  # Placeholder
    RETURN    ${iq}

Get Current D
    [Documentation]    Read D-axis current from ADC mock
    # TODO: Connect to Renode Python peripheral
    ${id} =    Evaluate    0.0  # Placeholder
    RETURN    ${id}

Get PWM Duty A
    [Documentation]    Read Phase A PWM duty from motor simulator
    # TODO: Connect to Renode Python peripheral
    ${duty} =    Evaluate    0.5  # Placeholder
    RETURN    ${duty}

Get PWM Duty B
    [Documentation]    Read Phase B PWM duty from motor simulator
    # TODO: Connect to Renode Python peripheral
    ${duty} =    Evaluate    0.5  # Placeholder
    RETURN    ${duty}

Get PWM Duty C
    [Documentation]    Read Phase C PWM duty from motor simulator
    # TODO: Connect to Renode Python peripheral
    ${duty} =    Evaluate    0.5  # Placeholder
    RETURN    ${duty}

Get Target Position
    [Documentation]    Read target position from motion planner
    # TODO: Extract from firmware state
    ${target} =    Evaluate    0.0  # Placeholder
    RETURN    ${target}

Get Target Velocity
    [Documentation]    Read target velocity from motion planner
    # TODO: Extract from firmware state
    ${target} =    Evaluate    0.0  # Placeholder
    RETURN    ${target}

Get Load Estimate
    [Documentation]    Read load estimate from adaptive controller
    # TODO: Extract from firmware state
    ${load} =    Evaluate    0.0  # Placeholder
    RETURN    ${load}

Get Motor Temperature
    [Documentation]    Read motor temperature from simulator
    # TODO: Connect to Renode Python peripheral
    ${temp} =    Evaluate    25.0  # Placeholder
    RETURN    ${temp}

Get Health Score
    [Documentation]    Read health score from health monitor
    # TODO: Extract from firmware state
    ${health} =    Evaluate    100.0  # Placeholder
    RETURN    ${health}

# Helper keywords
Create Directory
    [Documentation]    Create directory if it doesn't exist
    [Arguments]    ${dir_path}
    Run Keyword And Ignore Error    Evaluate    __import__('pathlib').Path('${dir_path}').mkdir(parents=True, exist_ok=True)

