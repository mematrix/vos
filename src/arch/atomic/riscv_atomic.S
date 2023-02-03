# riscv_atomic.S
# Atomic primitive implementation for RISC-V.

.option norvc

.section .text
.global compare_exchange64_lr_sc
compare_exchange64_lr_sc:
    # a0: address to compare.
    # a1: expected old value.
    # a2: new value to be written to the address.
    # ret: 0: CAS failed; 1: CAS success.
0:
    lr.d.aq t0, (a0)
    bne t0, a1, 1f
    sc.d.rl t1, a2, (a0)
    bnez t1, 0b
    li a0, 1
    ret
1:
    li a0, 0
    ret
