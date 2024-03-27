EXE = akimbo

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
	OLD := akimbo-$(VER).exe
	AVX2 := akimbo-$(VER)-avx2.exe
else
	NAME := $(EXE)
	OLD := akimbo-$(VER)
	AVX2 := akimbo-$(VER)-avx2
endif

rule:
	cargo rustc --release -- -C target-cpu=native --emit link=$(NAME)

release:
	cargo rustc --release -- --emit link=$(OLD)
	cargo rustc --release -- -C target-cpu=x86-64-v2 -C target-feature=+avx2 --emit link=$(AVX2)