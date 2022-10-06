# sched.S
# Scheduler finish work: restore the context and switch to the task.

.option norvc
.altmacro

.set NUM_GP_REGS, 32    # Number of registers per context
.set REG_SIZE, 8        # Register size (in bytes)

# Use macros for restoring multiple registers
.macro load_gp i, basereg=t6
    ld x\i, ((\i)*REG_SIZE)(\basereg)
.endm
.macro load_fp i, basereg=t6
    fld f\i, ((NUM_GP_REGS+(\i))*REG_SIZE)(\basereg)
.endm
.macro init_fp i
    fmv.d.x f\i, x0
.endm

.section .text
.global switch_to_task
switch_to_task:
    # Paramerters:
    # a0: TrapFrame (Task or Kernel) ptr.
    csrw sscratch, a0
    mv t6, a0
    # Set return address.
    ld t5, 512(a0)
    csrw sepc, t5

    # Check FS status.
    csrr t0, sstatus
    srli t1, t0, 13
    addi t1, t1, 3
    # If FS status is not OFF, restore the floating registers.
    beqz t1, 2f
    li t2, 1
    beq t1, t2, 1f
    # If SF status is not Initial, resotre from memory.
    .set i, 0
    .rept 32
        load_fp %i
        .set i, i+1
    .endr
    # Set the FS status to Clean.
    li t2, 0x2000
    csrrc x0, sstatus, t2
    j 2f

1:
    # FS status is Initial, set without load memory.
    .set i, 0
    .rept 32
        init_fp %i
        .set i, i+1
    .endr

2:
    # Restore all GP registers
    .set i, 1
    .rept 31
        load_gp %i
        .set i, i+1
    .endr
    # Since we ran this loop 31 times starting with i = 1,
    # the last one loaded t6 back to its original value.

    sret
