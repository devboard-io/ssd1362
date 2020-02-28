#!/bin/bash
FLASH_ADDRESS=$1
ELF=$2
arm-none-eabi-objcopy -O binary $ELF $ELF.bin
# cargo objcopy --bin $ELF --release -- -O binary $ELF.bin
st-flash --reset write $ELF.bin $FLASH_ADDRESS