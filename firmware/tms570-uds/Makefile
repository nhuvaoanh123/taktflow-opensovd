# taktflow-tms570-uds-fw — TMS570LC4357 bare-metal firmware
#
# Toolchain: TI Arm Clang (LLVM) 4.0.4.LTS (tiarmclang)
# Target:    Cortex-R5F, hard-float, big-endian (TMS570 default is BE32)

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
TI_CCS_ROOT  ?= /c/ti/ccs2041/ccs
COMPILER_DIR ?= $(TI_CCS_ROOT)/tools/compiler/ti-cgt-armllvm_4.0.4.LTS
DSLITE       ?= $(TI_CCS_ROOT)/ccs_base/DebugServer/bin/DSLite.exe

# Single-front-end toolchain: tiarmclang (LLVM driver) handles C, C++, and
# GNU-syntax .S assembly through its integrated assembler. tiarmasm is NOT
# used — all .S files in this tree use GNU syntax, so the LLVM MC assembler
# bundled with tiarmclang accepts them directly.
CC      := $(COMPILER_DIR)/bin/tiarmclang.exe
LD      := $(COMPILER_DIR)/bin/tiarmclang.exe
HEX     := $(COMPILER_DIR)/bin/tiarmhex.exe
DIS     := $(COMPILER_DIR)/bin/tiarmdis.exe
RUNTIME := $(COMPILER_DIR)/lib

# ---------------------------------------------------------------------------
# Project
# ---------------------------------------------------------------------------
TARGET    := taktflow-tms570-uds
BUILD_DIR := build

BSP_SRC_DIR := bsp/src
BSP_INC_DIR := bsp/include
APP_SRC_DIR := src

BSP_C_SRCS := $(wildcard $(BSP_SRC_DIR)/*.c)
APP_C_SRCS := $(wildcard $(APP_SRC_DIR)/*.c)
APP_S_SRCS := $(wildcard $(APP_SRC_DIR)/*.S)

C_SRCS := $(BSP_C_SRCS) $(APP_C_SRCS)
S_SRCS := $(APP_S_SRCS)

C_OBJS := $(patsubst %.c,$(BUILD_DIR)/%.o,$(C_SRCS))
S_OBJS := $(patsubst %.S,$(BUILD_DIR)/%.o,$(S_SRCS))
OBJS   := $(C_OBJS) $(S_OBJS)

LINKER_CMD := linker.cmd

# ---------------------------------------------------------------------------
# Flags — Cortex-R5F, hard-float, VFPv3-d16
# ---------------------------------------------------------------------------
CPU_FLAGS := -mcpu=cortex-r5 -mfloat-abi=hard -mfpu=vfpv3-d16 -mbig-endian -marm

COMMON_FLAGS := $(CPU_FLAGS) \
                -Os \
                -g \
                -ffunction-sections \
                -fdata-sections \
                -Wall -Wextra \
                -I$(BSP_INC_DIR) \
                -I$(APP_SRC_DIR)

CFLAGS := $(COMMON_FLAGS) -std=c11
# GNU-syntax .S files go through tiarmclang's integrated LLVM assembler with
# the same CPU flags as the C compile — no separate assembler invocation.
SFLAGS := $(CPU_FLAGS) -g -I$(BSP_INC_DIR) -I$(APP_SRC_DIR)

LDFLAGS  := $(CPU_FLAGS) \
            -Wl,--be32 \
            -Wl,--map_file=$(BUILD_DIR)/$(TARGET).map \
            -Wl,--rom_model \
            -Wl,--unused_section_elimination=on \
            -Wl,--reread_libs \
            -Wl,--diag_wrap=off \
            -Wl,--display_error_number \
            -Wl,--emit_warnings_as_errors
# --be32 is MANDATORY. The compiler's -mbig-endian produces BE-endian object
# files, but the linker defaults to BE-8 (instruction bytes little-endian
# inside a big-endian word). TMS570LC43xx fetches instructions in BE-32
# order; without --be32 every instruction is byte-swapped → instant abort
# before any code runs, LED preamble included.

# tiarmclang driver auto-links the runtime library for the selected target
# triple (armv7reb-ti-none-eabihf for Cortex-R5 big-endian hard-float).
# No explicit -lc needed.
LIBS :=

# ---------------------------------------------------------------------------
# Targets
# ---------------------------------------------------------------------------
.PHONY: all clean flash dis size

all: $(BUILD_DIR)/$(TARGET).out $(BUILD_DIR)/$(TARGET).hex

$(BUILD_DIR)/%.o: %.c
	@mkdir -p $(dir $@)
	"$(CC)" $(CFLAGS) -c $< -o $@

$(BUILD_DIR)/%.o: %.S
	@mkdir -p $(dir $@)
	"$(CC)" $(SFLAGS) -c $< -o $@

$(BUILD_DIR)/$(TARGET).out: $(OBJS) $(LINKER_CMD)
	"$(LD)" $(LDFLAGS) \
		-o $@ $(OBJS) $(LINKER_CMD) $(LIBS)

$(BUILD_DIR)/$(TARGET).hex: $(BUILD_DIR)/$(TARGET).out
	"$(HEX)" --ti_txt -o $@ $<

dis: $(BUILD_DIR)/$(TARGET).out
	"$(DIS)" $< > $(BUILD_DIR)/$(TARGET).dis

size: $(BUILD_DIR)/$(TARGET).out
	@grep -E "^(FLASH0|FLASH1|RAM|STACKS)" $(BUILD_DIR)/$(TARGET).map || true

flash: $(BUILD_DIR)/$(TARGET).out
	"$(DSLITE)" load \
		-c $(APP_SRC_DIR)/../TMS570LC4357_XDS110.ccxml \
		-f $<

clean:
	rm -rf $(BUILD_DIR)
