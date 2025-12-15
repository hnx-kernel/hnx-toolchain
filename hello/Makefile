ELF := hello.elf
OBJ := hello.o
LD_SCRIPT := link-elf.ld
RUSTC ?= rustc
TARGET ?= aarch64-unknown-none
RUSTFLAGS ?= -C opt-level=2 -C panic=abort -C relocation-model=static

.PHONY: all clean

all: $(OBJ) $(ELF)

$(ELF): $(OBJ) $(LD_SCRIPT)
	rust-lld -flavor gnu -T $(LD_SCRIPT) $(OBJ) -o $(ELF)

$(OBJ): src/main.rs
	$(RUSTC) --target $(TARGET) $(RUSTFLAGS) --emit=obj -o $(OBJ) $<

clean:
	rm -f $(ELF)
