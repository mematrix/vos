# m_trap.S
# Machine mode assembly-level trap handler.

.option norvc

.section .text
.global m_asm_trap_handler
# This must be aligned by 4 since the last two bits of the mtvec register do not contribute to
# the address of this vector.
.align 4
m_asm_trap_handler:
    # We got here when the CPU is interrupted for any reason and the interrupts are not delegated
    # to the S-mode. For example: an `ecall` from S-mode.
    # Now we only handle these traps in M-mode: ecall from S-mode.
    # So the handler does not save any register, instead, use a calling convention like function
    # call. such as register a0-a7, t0-t6 are caller saved registers.
    csrr t1, mcause
    li t2, 0x09
    bne t1, t2, unexpect

    # Handle ecall from S-mode
    # a7: syscall function number
    #   0: read hartid
    #   1: set next timer
    li t3, 2
    bgeu a7, t3, unexpect
    # Jump the trap table
    la t4, trap_table
    slli t5, a7, 3  # num * 8 -> jump table, each table entry contains 2 instructions.
    add t4, t4, t5
    jalr 0(t4)

    # update pc
    csrr t0, mepc
    addi t0, t0, 4
    csrw mepc, t0

    mret

trap_table:
    # read_hartid
    csrr a0, mhartid
    ret
    # set_next_timer
    j set_next_timer
    nop

# Syscalls (more than 2 instructions)
set_next_timer:
    # a0: the delta tick count next timer interrupt occurs.
    rdtime t0
    add t2, t0, a0
    #la t1, mtimecmp
    #sd t2, 0(t1)
    csrw stimecmp, t2
    ret

unexpect:
    wfi
    j unexpect
