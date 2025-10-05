*** Settings ***
Documentation     Basic firmware startup tests for STM32G431CB FOC controller
Suite Setup       Setup
Suite Teardown    Teardown
Test Setup        Reset Emulation
Resource          ${RENODEKEYWORDS}

*** Variables ***
${UART}                     sysbus.usart1
${SCRIPT}                   ${CURDIR}/../stm32g431_foc.resc
${ELF}                      ${CURDIR}/../../target/thumbv7em-none-eabihf/release/joint_firmware
${LOG_TIMEOUT}              5

*** Test Cases ***
Should Boot And Show Banner
    [Documentation]         Firmware should boot and print initialization banner
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Wait for startup messages
    Wait For Line On Uart   CLN17 v2.0 Joint Firmware           timeout=${LOG_TIMEOUT}
    Wait For Line On Uart   STM32G431CB                         timeout=${LOG_TIMEOUT}

Should Initialize System
    [Documentation]         System should complete initialization
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Check initialization steps
    Wait For Line On Uart   Joint Firmware Initialization       timeout=${LOG_TIMEOUT}
    Wait For Line On Uart   System Ready                        timeout=${LOG_TIMEOUT}

Should Start Heartbeat
    [Documentation]         Heartbeat should start ticking
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Wait for at least 3 heartbeats
    Wait For Line On Uart   System heartbeat: 1 sec             timeout=2
    Wait For Line On Uart   System heartbeat: 2 sec             timeout=2
    Wait For Line On Uart   System heartbeat: 3 sec             timeout=2

Should Initialize PWM
    [Documentation]         PWM should be ready for motor control
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Wait for FOC task which initializes PWM
    Wait For Line On Uart   FOC task started                    timeout=${LOG_TIMEOUT}

Should Initialize CAN
    [Documentation]         FDCAN1 should be initialized
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # CAN task should spawn
    Wait For Line On Uart   CAN task started                    timeout=${LOG_TIMEOUT}

