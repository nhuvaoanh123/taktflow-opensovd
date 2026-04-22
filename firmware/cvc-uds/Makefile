# taktflow-cvc-uds-fw - STM32G474RE HIL-oriented firmware

SHELL := C:/Program Files/Git/bin/bash.exe

TARGET := taktflow-cvc-uds
BUILD_DIR := build
APP_SRC_DIR := src
LINKER_SCRIPT := stm32g474re.ld
OPENOCD_CFG := openocd.cfg
EMBEDDED_ROOT ?= ../../../taktflow-embedded-production
CUBEMX_DIR := $(EMBEDDED_ROOT)/firmware/ecu/cvc/cfg
HAL_DIR := $(CUBEMX_DIR)/Drivers/STM32G4xx_HAL_Driver
CMSIS_DIR := $(CUBEMX_DIR)/Drivers/CMSIS

TOOLCHAIN_PREFIX ?= C:/tools/msys64/ucrt64/bin/arm-none-eabi-
CC := $(TOOLCHAIN_PREFIX)gcc.exe
OBJCOPY := $(TOOLCHAIN_PREFIX)objcopy.exe
SIZE := $(TOOLCHAIN_PREFIX)size.exe
OPENOCD ?= C:/tools/msys64/ucrt64/bin/openocd.exe

CPU_FLAGS := -mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16
COMMON_FLAGS := $(CPU_FLAGS) \
	-ffreestanding \
	-fno-builtin \
	-ffunction-sections \
	-fdata-sections \
	-DSTM32G474xx \
	-DUSE_HAL_DRIVER \
	-Wall \
	-Wextra \
	-Werror=implicit-function-declaration \
	-I$(APP_SRC_DIR) \
	-I$(CUBEMX_DIR)/Core/Inc \
	-I$(HAL_DIR)/Inc \
	-I$(HAL_DIR)/Inc/Legacy \
	-I$(CMSIS_DIR)/Device/ST/STM32G4xx/Include \
	-I$(CMSIS_DIR)/Include
CFLAGS := $(COMMON_FLAGS) -Og -g3 -std=c11
LDFLAGS := $(CPU_FLAGS) \
	-T$(LINKER_SCRIPT) \
	-nostartfiles \
	-Wl,--gc-sections \
	-Wl,-Map=$(BUILD_DIR)/$(TARGET).map \
	--specs=nano.specs \
	--specs=nosys.specs \
	-lm

HAL_SRCS := \
	$(HAL_DIR)/Src/stm32g4xx_hal.c \
	$(HAL_DIR)/Src/stm32g4xx_hal_cortex.c \
	$(HAL_DIR)/Src/stm32g4xx_hal_rcc.c \
	$(HAL_DIR)/Src/stm32g4xx_hal_rcc_ex.c \
	$(HAL_DIR)/Src/stm32g4xx_hal_gpio.c \
	$(HAL_DIR)/Src/stm32g4xx_hal_exti.c \
	$(HAL_DIR)/Src/stm32g4xx_hal_pwr.c \
	$(HAL_DIR)/Src/stm32g4xx_hal_pwr_ex.c \
	$(HAL_DIR)/Src/stm32g4xx_hal_fdcan.c \
	$(HAL_DIR)/Src/stm32g4xx_hal_flash.c \
	$(HAL_DIR)/Src/stm32g4xx_hal_flash_ex.c \
	$(HAL_DIR)/Src/stm32g4xx_hal_flash_ramfunc.c \
	$(CUBEMX_DIR)/Core/Src/system_stm32g4xx.c

APP_C_SRCS := $(wildcard $(APP_SRC_DIR)/*.c) $(HAL_SRCS)
OBJS := $(patsubst %.c,$(BUILD_DIR)/%.o,$(APP_C_SRCS))

.PHONY: all clean flash size

all: $(BUILD_DIR)/$(TARGET).elf $(BUILD_DIR)/$(TARGET).bin

$(BUILD_DIR)/%.o: %.c
	@mkdir -p $(dir $@)
	"$(CC)" $(CFLAGS) -c $< -o $@

$(BUILD_DIR)/$(TARGET).elf: $(OBJS) $(LINKER_SCRIPT)
	@mkdir -p $(BUILD_DIR)
	"$(CC)" $(OBJS) $(LDFLAGS) -o $@

$(BUILD_DIR)/$(TARGET).bin: $(BUILD_DIR)/$(TARGET).elf
	"$(OBJCOPY)" -O binary $< $@

size: $(BUILD_DIR)/$(TARGET).elf
	"$(SIZE)" $<

flash: $(BUILD_DIR)/$(TARGET).bin $(OPENOCD_CFG)
	"$(OPENOCD)" -f $(OPENOCD_CFG) -c "program $(BUILD_DIR)/$(TARGET).bin verify reset exit 0x08000000"

clean:
	rm -rf $(BUILD_DIR)
