EXE = akimbo

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
	OLD := akimbo-0.$(VER).0.exe
	AVX2 := akimbo-0.$(VER).0-avx2.exe
else
	NAME := $(EXE)
	OLD := akimbo-0.$(VER).0
	AVX2 := akimbo-0.$(VER).0-avx2
endif

rule:
	cargo rustc --release -- -C target-cpu=native --emit link=$(NAME)

datagen:
	cargo rustc --release --features=datagen -- -C target-cpu=native --emit link=$(NAME)

release:
	cargo rustc --release -- --emit link=$(OLD)
	cargo rustc --release -- -C target-cpu=x86-64-v2 -C target-feature=+avx2 --emit link=$(AVX2)