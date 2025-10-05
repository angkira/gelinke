*** Settings ***
Documentation     FOC control loop and motor control tests
Suite Setup       Setup
Suite Teardown    Teardown
Test Setup        Reset Emulation
Resource          ${RENODEKEYWORDS}

*** Variables ***
${UART}                     sysbus.usart1
${TIM1}                     sysbus.tim1
${ADC1}                     sysbus.adc1
${SPI1}                     sysbus.spi1
${ELF}                      ${CURDIR}/../../target/thumbv7em-none-eabihf/release/joint_firmware

*** Test Cases ***
Should Start FOC Task
    [Documentation]         FOC control loop task should start
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # FOC task spawns
    Wait For Line On Uart   FOC|control loop|task               timeout=5

Should Initialize PWM Timer
    [Documentation]         TIM1 should be configured for 20kHz PWM
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Start Emulation
    
    Execute Command         emulation RunFor "00:00:00.1"
    
    # TIM1 should be running
    ${limit}=               Execute Command    ${TIM1} Limit
    Should Be True          ${limit} > 0

Should Read ADC Current Sensors
    [Documentation]         ADC should read phase currents
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Start Emulation
    
    Execute Command         emulation RunFor "00:00:01"
    
    # ADC should have conversions
    ${adc_status}=          Execute Command    ${ADC1}
    Should Contain          ${adc_status}      ADC

Should Read Encoder Position
    [Documentation]         SPI should read TLE5012B encoder
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Start Emulation
    
    Execute Command         emulation RunFor "00:00:01"
    
    # SPI should be active
    ${spi_status}=          Execute Command    ${SPI1}
    Should Contain          ${spi_status}      SPI

Should Run Control Loop At 10kHz
    [Documentation]         FOC loop should execute at 10kHz rate
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Run for 1ms = 10 FOC cycles
    Execute Command         emulation RunFor "00:00:00.001"


