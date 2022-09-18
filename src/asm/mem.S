// mem.S
// Importation of linker symbols

.section .rodata
.global HEAP_START
HEAP_START: .dword _heap_start

.global HEAP_SIZE
HEAP_SIZE: .dword _heap_size

.global TEXT_START
TEXT_START: .dword _text_start

.global TEXT_END
TEXT_END: .dword _text_end

.global DATA_START
DATA_START: .dword _data_start

.global DATA_END
DATA_END: .dword _data_end

.global RODATA_START
RODATA_START: .dword _rodata_start

.global RODATA_END
RODATA_END: .dword _rodata_end

.global BSS_START
BSS_START: .dword _bss_start

.global BSS_END
BSS_END: .dword _bss_end

.global KERNEL_STACK_START
KERNEL_STACK_START: .dword _stack_start

.global KERNEL_STACK_END
KERNEL_STACK_END: .dword _stack_end


.section .data
.global KERNEL_TABLE
KERNEL_TABLE: .dword 0


.global write_on_stack
write_on_stack:
    # a0: Size to alloc on stack.
    # a1: Function to write data: fn(_:*mut u8, _:*const ()) -> *mut u8.
    #     The fn must return the first param value.
    # a2: User data forward to the callback fn. *const ().
    mv t0, sp
    sub sp, sp, a0
    mv t1, a1
    mv a0, t0
    mv a1, a2
    jalr t1
    # Use the returned value to restore the sp register
    mv sp, a0
    ret
