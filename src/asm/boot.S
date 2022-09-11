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
    la t0, _bss_start
    la t1, _bss_end
    bgeu t0, t1, 2f
1:
    sd zero, (t0)
    addi t0, t0, 8
    bltu t0, t1, 1b

2:
    # Init sp register
    la sp, _stack_end
    # We use mret here, update mstatus register: set mPP=0b11, Disable interrupts.
    li t0, (0b11 << 11)
    csrw mstatus, t0
    # mepc register saves the address that mret will jump.
    # From the firmware (qemu ROM code), a0 is set to the hart_id and a1 is set to
    # the address of the devicetree blob.
    la t1, m_init
    csrw mepc, t1
    # Set the machine mode trap vector base address.
    la t2, asm_trap_vector
    csrw mtvec, t2

    la ra, 3f
    mret

3:
    # We set the return address (ra above) to this label. When m_init() is finished
    # in Rust, it will return here.

    # Set machine mode interrupt flags. Although we will be not in the M-mode after
    # the sret. set mPIE and mIE.
    li t0, (0b01 << 11) | (1 << 7) | (1 << 3)
    csrw mstatus, t0
    # Machine interrupt enable register.
    li t0, (1 << 3) | (1 << 7) | (1 << 11)
    csrw mie, t0

    # Set the PMP config. Simplely allow all address.
    li t0, (0b01 << 3) | 0b111
    csrw pmpcfg0, t0
    li t0, 0x100000000
    csrw pmpaddr0, t0

    # Now set the S-mode CSRs.

    # Setting `sstatus` (supervisor status) register:
    # 1 << 8    : Supervisor's previous protection mode is 1 (SPP=1 [Supervisor]).
    # 1 << 5    : Supervisor's previous interrupt-enable bit is 1 (SPIE=1 [Enabled]).
    # 1 << 1    : Supervisor's interrupt-enable bit will be set to 1 after sret.
    li t0, (1 << 8) | (1 << 5)
    csrw sstatus, t0
    # The kmain Rust fn will run in S-mode.
    la t1, kmain
    csrw sepc, t1
    # Setting `mideleg` (machine interrupt delegate) register:
    # 1 << 1   : Software interrupt delegated to supervisor mode
    # 1 << 5   : Timer interrupt delegated to supervisor mode
    # 1 << 9   : External interrupt delegated to supervisor mode
    # By default all traps (interrupts or exceptions) automatically
    # cause an elevation to the machine privilege mode (mode 3).
    # When we delegate, we're telling the CPU to only elevate to
    # the supervisor privilege mode (mode 1)
    li t2, (1 << 1) | (1 << 5) | (1 << 9)
    csrw mideleg, t2
    # Setting `sie` (supervisor interrupt enable) register:
    # This register takes the same bits as mideleg
    # 1 << 1    : Supervisor software interrupt enable (SSIE=1 [Enabled])
    # 1 << 5    : Supervisor timer interrupt enable (STIE=1 [Enabled])
    # 1 << 9    : Supervisor external interrupt enable (SEIE=1 [Enabled])
    csrw sie, t2
    # Setting `medeleg` (machine exception delegate) register:
    # Delegate all available exceptions.
    # 0b 1 0 1 1, 0 0 0 1, 1 1 1 1, 1 1 1 1
    #   15......,11......, 7......, 3......
    # bit 14 and bit 10 are reserved;
    # bit 11 are read-only (M-mode trap cannot be delegated to S,U mode);
    # bit 10 is not set because we need to handle the S-mode ecall on the M-mode.
    li t3, 0xb1ff
    csrw medeleg, t3
    # Setting `stvec` (supervisor trap vector) register:
    # Essentially this is a function pointer, but the last two bits can be 00 or 01
    # 00        : All exceptions set pc to BASE
    # 01        : Asynchronous interrupts set pc to BASE + 4 x scause
    la t4, asm_trap_vector
    csrw stvec, t4
    # m_init() is required to return back the SATP value (including MODE) via a0
    csrw satp, a0
    # Force the CPU to take our SATP register.
    # To be efficient, if the address space identifier (ASID) portion of SATP is already
    # in cache, it will just grab whatever's in cache. However, that means if we've updated
    # it in memory, it will be the old table. So, sfence.vma will ensure that the MMU always
    # grabs a fresh copy of the SATP register and associated tables.
    sfence.vma

    la ra, 5f
    # sret will put us in Supervisor mode and re-enable interrupts.
    sret

5:
    wfi
    j 5b
