# boot.S
# bootloader for vOS

.option norvc
.section .data

.section .text.init
.global _start
_start:

.option push
.option norelax
    la gp, _global_pointer
.option pop

    # SATP should be zero. let's make sure.
    csrw satp, zero
    # Any hardware threads (hart) that are not bootstrapping need to wait for an IPI
    csrr t0, mhartid
    bnez t0, 5f

    # Set all bytes in the BSS section to zero.
    la a0, _bss_start
    la a1, _bss_end
    bgeu a0, a1, 2f
1:
    sd zero, (a0)
    addi a0, a0, 8
    bltu a0, a1, 1b

2:
    # Init sp register
    la sp, _stack_end
    # We use mret here, update mstatus register: set mPP=0b11, set mPIE and mIE
    li t0, (0b11 << 11) | (1 << 7) | (1 << 3)
    csrw mstatus, t0
    la t1, m_init
    csrw mepc, t1
    la t2, asm_trap_vector
    csrw mtvec, t2
    li t3, (1 << 3) | (1 << 7) | (1 << 11)
    csrw mie, t3
    la ra, 3f
    mret

3:
    li t0, (0b11 << 11) | (1 << 7) | (1 << 3)
    csrw mstatus, t0
    la t1, kmain
    csrw mepc, t1
    la ra, 5f
    mret

5:
    wfi
    j 5b
