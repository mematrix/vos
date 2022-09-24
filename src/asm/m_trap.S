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
    mret
