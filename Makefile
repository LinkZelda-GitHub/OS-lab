OVMF := assets/OVMF.fd
ESP := esp
BUILD_ARGS :=
QEMU_ARGS := -m 96M
QEMU_OUTPUT ?= -nographic
MODE ?= release
CUR_PATH := $(shell pwd)
APP_PATH := $(CUR_PATH)/crates/app
DBG_INFO ?= false

.PHONY: build run debug clean launch intdbg gui \
	target/x86_64-unknown-uefi/$(MODE)/ysos_boot.efi \
	target/x86_64-unknown-none/$(PROFILE)/ysos_kernel \
	target/x86_64-unknown-ysos/$(MODE)

run: build launch

# GUI mode: open QEMU window with VGA display, serial output on terminal.
# Prefers SDL display (works with WSLg/X11), falls back to VNC if needed.
# To override: make gui DISPLAY_BACKEND=gtk
DISPLAY_BACKEND ?= sdl
ifeq ($(DISPLAY_BACKEND),vnc)
gui: QEMU_OUTPUT = -vga std -serial stdio -vnc 0.0.0.0:0
else
gui: QEMU_OUTPUT = -vga std -serial stdio -display $(DISPLAY_BACKEND)
endif
gui: build launch

launch:
	@qemu-system-x86_64 \
		-bios ${OVMF} \
		-net none \
		$(QEMU_ARGS) \
		$(QEMU_OUTPUT) \
		-drive format=raw,file=fat:rw:${ESP}
APPS := $(filter-out $(APP_PATH),$(patsubst $(APP_PATH)/%, %, $(APPS)))
APPS := $(filter-out config,$(APPS))
APPS := $(filter-out .cargo,$(APPS))

# Only add debug info for kernel
# this is required for VSCode GUI debugging
ifeq (${DBG_INFO}, true)
	PROFILE = release-with-debug
	PROFILE_ARGS = --profile=release-with-debug
else
	PROFILE = ${MODE}
	PROFILE_ARGS = $(BUILD_ARGS)
endif

ifeq (${MODE}, release)
	BUILD_ARGS := --release
endif

run: build launch

launch:
	@qemu-system-x86_64 \
		-bios ${OVMF} \
		-net none \
		$(QEMU_ARGS) \
		$(QEMU_OUTPUT) \
		-drive format=raw,file=fat:rw:${ESP}

intdbg:
	@qemu-system-x86_64 \
		-bios ${OVMF} \
		-net none \
		$(QEMU_ARGS) \
		$(QEMU_OUTPUT) \
		-drive format=raw,file=fat:rw:${ESP} \
		-no-reboot -d int,cpu_reset

debug:
	@qemu-system-x86_64 \
		-bios ${OVMF} \
		-net none \
		$(QEMU_ARGS) \
		$(QEMU_OUTPUT) \
		-drive format=raw,file=fat:rw:${ESP} \
		-s -S

clean:
	@cargo clean

list:
	@for dir in $(APPS); do echo $$dir || exit; done

build: $(ESP)

$(ESP): $(ESP)/EFI/BOOT/BOOTX64.EFI $(ESP)/KERNEL.ELF $(ESP)/EFI/BOOT/boot.conf $(ESP)/APP

$(ESP)/EFI/BOOT/BOOTX64.EFI: target/x86_64-unknown-uefi/$(MODE)/ysos_boot.efi
	@mkdir -p $(@D)
	cp $< $@

$(ESP)/EFI/BOOT/boot.conf: crates/kernel/config/boot.conf
	@mkdir -p $(@D)
	cp $< $@

$(ESP)/KERNEL.ELF: target/x86_64-unknown-none/$(PROFILE)/ysos_kernel
	@mkdir -p $(@D)
	cp $< $@

$(ESP)/APP: target/x86_64-unknown-ysos/$(MODE)
	@for app in $(APPS); do \
		mkdir -p $(ESP)/APP; \
		cp $</ysos_$$app $(ESP)/APP/$$app; \
	done


target/x86_64-unknown-uefi/$(MODE)/ysos_boot.efi: crates/boot
	cd crates/boot && cargo build $(BUILD_ARGS)

target/x86_64-unknown-none/$(PROFILE)/ysos_kernel: crates/kernel
	cd crates/kernel && cargo build $(PROFILE_ARGS) -Z json-target-spec

target/x86_64-unknown-ysos/$(MODE):
	@for app in $(APPS); do \
		echo "Building $$app"; \
		cd $(APP_PATH)/$$app && cargo build $(BUILD_ARGS) -Z json-target-spec || exit; \
	done
