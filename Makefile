PREFIX ?= msp430-elf-

CFLAGS += -Wall -Wextra
LDFLAGS += -T msp430fr6972.ld 

SRCS := $(shell find . -name "*.c")
OBJS := $(subst .c,.o,$(SRCS))

firmware.out: $(OBJS)
	$(PREFIX)$(CC) $(LDFLAGS) -o $@ $<

%.o : %.c
	$(PREFIX)$(CC) -I include -c $(CFLAGS) $< -o $@

%PHONY: clean
clean:
	rm -f $(OBJS)
