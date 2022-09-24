# trap.S
# Assembly-level trap handler of Supervisor mode.

.option norvc

.section .text
.global asm_trap_handler
# This must be aligned by 4 since the last two bits of the stvec register do not contribute to
# the address of this vector.
.align 4
asm_trap_handler:
    # We get here when the CPU is interrupted for any reason and the interrupts are delegated to
    # the S-mode. Such as `ecall` from U-mode, timer interrupt, etc.
    sret
