# POSIX unit-test harness for firmware/cvc-uds/src/ota.c
#
# Usage:
#   make         # build + run
#   make build   # build only
#   make clean   # remove artifacts

CC      ?= gcc
FW_SRC  := ../../../firmware/cvc-uds/src
BUILD   := build
TARGET  := $(BUILD)/test_ota
SRCS    := test_ota.c stubs/hal_fake.c $(FW_SRC)/ota.c $(FW_SRC)/sha256.c
CFLAGS  := -Wall -Wextra -std=c11 -O0 -g \
           -DPOSIX_OTA_TEST \
           -I stubs -I $(FW_SRC) \
           -include stubs/main.h

.PHONY: all build run clean
all: run

build: $(TARGET)

$(TARGET): $(SRCS) stubs/main.h | $(BUILD)
	$(CC) $(CFLAGS) -o $@ $(SRCS)

$(BUILD):
	mkdir -p $(BUILD)

run: $(TARGET)
	$(TARGET)

clean:
	rm -rf $(BUILD)
