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
    # a1: Function to write data: fn(*mut u8, usize, *const ()).
    #     First param is the stack mem ptr and the second param is the alloc size.
    # a2: User data forward to the callback fn. *const ().
    sd ra, -8(sp)
    sd fp, -16(sp)
    mv fp, sp
    addi sp, sp, -16
    # Alloc memory of `size` bytes.
    sub sp, sp, a0
    mv t1, a1
    mv a1, a0
    mv a0, sp
    # Keep a2 param
    jalr t1
    # Restore the sp and return stack ptr.
    mv a0, sp
    mv sp, fp
    ld ra, -8(sp)
    ld fp, -16(sp)
    ret
