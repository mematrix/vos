# trap.S
# Assembly-level trap handler of Supervisor mode.

.option norvc
.altmacro

.set NUM_GP_REGS, 32  # Number of registers per context
.set REG_SIZE, 8   # Register size (in bytes)

# Use macros for saving and restoring multiple registers
.macro save_gp i, basereg=t6
    sd	x\i, ((\i)*REG_SIZE)(\basereg)
.endm
.macro load_gp i, basereg=t6
    ld	x\i, ((\i)*REG_SIZE)(\basereg)
.endm
.macro save_fp i, basereg=t6
    fsd	f\i, ((NUM_GP_REGS+(\i))*REG_SIZE)(\basereg)
.endm
.macro load_fp i, basereg=t6
    fld	f\i, ((NUM_GP_REGS+(\i))*REG_SIZE)(\basereg)
.endm

.section .text
.global asm_trap_handler
# This must be aligned by 4 since the last two bits of the stvec register do not contribute to
# the address of this vector.
.align 4
asm_trap_handler:
    # We get here when the CPU is interrupted for any reason and the interrupts are delegated to
    # the S-mode. Such as `ecall` from U-mode, timer interrupt, etc.

    # All registers are volatile here, we need to save them before we do anything.
    # sscratch saved the trap frame pointer of the running thread context.
    # csrrw will atomically swap t6 into sscratch and the old value of sscratch into t6. This is
    # nice because we just switched values and didn't destroy anything -- all atomically!
    csrrw t6, sscratch, t6
    # in sched/sched we have a structure `TaskTrapFrame` of:
    #  32 gp regs		0
    #  32 fp regs		256
    # We use t6 as the temporary register because it is the very bottom register (x31)
    .set i, 0
    .rept 31
        save_gp	%i
        .set i, i+1
    .endr

    # Save the actual t6 register, which we swapped into sscratch.
    # Now t5 save the sscratch (trap frame).
    mv t5, t6
    csrr t6, sscratch
    save_gp 31, t5

    # Restore the trap frame into sscratch.
    csrw sscratch, t5

    # Check floating regs status: if FS[1:0] flag is dirty, we need store the floating registers.
    csrr t1, sstatus
    srli t0, t1, 13 # FS[1:0] flag.
    andi t0, t0, 3
    li t3, 3
    bne t0, t3, 1f
    # Save floating point registers
    .set i, 0
    .rept 32
        save_fp %i, t5
        .set i, i+1
    .endr
1:
    # Get ready to go into Rust (sched/trap.rs).
    csrr a0, sepc
    # Save pc (see TaskTrapFrame definition).
    sd a0, 512(t5)
    csrr a1, stval
    csrr a2, scause
    #csrr a3, mhartid
    csrr a3, sstatus
    csrr a4, sscratch   # t5
    # Load CPU stack. (see TrapStackFrame in smp/cpu_stack.rs)
    ld t0, 520(t5)
    ld sp, 0(t0)
    ld gp, 8(t0)
    ld tp, 16(t0)
    # tp points to current hart's CpuInfo.
    mv a5, tp
    # Rust trap handler only need to keep sscratch unchanged before return.
    call handle_trap

    # Return from Rust trap handler. The return address will be returned via a0.
    # Restore all registers and return.

    csrw sepc, a0
    # Now load the trap frame back to t6
    csrr t6, sscratch

    csrr t1, sstatus
    srli t0, t1, 13
    andi t0, t0, 3
    li t3, 3
    bne t0, t3, 2f
    .set i, 0
    .rept 32
        load_fp %i
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
