unexport CARGO_INCREMENTAL

TARGET_NAME=thumbv7m-none-eabi
PROJECT_NAME=controller


release:
	make rb
	make gdb_release

all:
	make build
	make gdb


rb:
	cargo build --release

build:
	cargo build

openocd:
	openocd -f bluepill.cfg

discovery_openocd:
	openocd -f interface/stlink-v2-1.cfg -f target/stm32f3x.cfg


gdb:
	arm-none-eabi-gdb target/${TARGET_NAME}/debug/${PROJECT_NAME}

gdb_release:
	arm-none-eabi-gdb target/${TARGET_NAME}/release/${PROJECT_NAME}

esp_monitor:
	stty -F ${DEV} inlcr
	stty -F ${DEV} onlcr
	screen ${DEV} 9600

doc:
	cargo doc
