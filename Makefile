EXE = akimbo

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
else
	NAME := $(EXE)
endif

rule:
	cargo rustc --release -- -C target-cpu=native --emit link=$(NAME)
