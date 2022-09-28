//! Memory maps in the QEMU virt machine.

use crate::constant::{ORDER_1GB, ORDER_2MB};
use crate::util::align::{align_down, align_up};


/* QEMU RISC-V memory maps (qemu/hw/riscv/virt.c)
static const MemMapEntry virt_memmap[] = {
    [VIRT_DEBUG] =        {        0x0,         0x100 },
    [VIRT_MROM] =         {     0x1000,        0xf000 },
    [VIRT_TEST] =         {   0x100000,        0x1000 },
    [VIRT_RTC] =          {   0x101000,        0x1000 },
    [VIRT_CLINT] =        {  0x2000000,       0x10000 },
    [VIRT_ACLINT_SSWI] =  {  0x2F00000,        0x4000 },
    [VIRT_PCIE_PIO] =     {  0x3000000,       0x10000 },
    [VIRT_PLATFORM_BUS] = {  0x4000000,     0x2000000 },
    [VIRT_PLIC] =         {  0xc000000, VIRT_PLIC_SIZE(VIRT_CPUS_MAX * 2) },
    [VIRT_APLIC_M] =      {  0xc000000, APLIC_SIZE(VIRT_CPUS_MAX) },
    [VIRT_APLIC_S] =      {  0xd000000, APLIC_SIZE(VIRT_CPUS_MAX) },
    [VIRT_UART0] =        { 0x10000000,         0x100 },
    [VIRT_VIRTIO] =       { 0x10001000,        0x1000 },
    [VIRT_FW_CFG] =       { 0x10100000,          0x18 },
    [VIRT_FLASH] =        { 0x20000000,     0x4000000 },
    [VIRT_IMSIC_M] =      { 0x24000000, VIRT_IMSIC_MAX_SIZE },
    [VIRT_IMSIC_S] =      { 0x28000000, VIRT_IMSIC_MAX_SIZE },
    [VIRT_PCIE_ECAM] =    { 0x30000000,    0x10000000 },
    [VIRT_PCIE_MMIO] =    { 0x40000000,    0x40000000 },
    [VIRT_DRAM] =         { 0x80000000,           0x0 },
};
*/

const VIRT_CPUS_MAX: usize = 1usize << 9;
const APLIC_MIN_SIZE: usize = 0x4000;
// const VIRT_IMSIC_MAX_SIZE: usize =

const fn aplic_size(cpus: usize) -> usize {
    APLIC_MIN_SIZE + align_up(cpus * 32, 14)
}

const fn virt_plic_size(cpus: usize) -> usize {
    const VIRT_PLIC_CONTEXT_BASE: usize = 0x200000;
    const VIRT_PLIC_CONTEXT_STRIDE: usize = 0x1000;

    VIRT_PLIC_CONTEXT_BASE + (cpus * 2) * VIRT_PLIC_CONTEXT_STRIDE
}

// 2M = 0x20_0000 = 1 << 21
/// Memory map list for page level 1 (2MiB per entry).
static VIRT_MEM_MAP_2MB: [(usize, usize); 10] = [
    // (0x100000, 0x2000),     // TEST and RTC. Lower than 2M, ignore to map
    (0x2000000, 0x10000),   // CLINT
    (align_down(0x2F00000, ORDER_2MB), 0x4000), // ACLINT_SSWI
    (0x3000000, 0x10000),   // PCIE_PIO
    (0x4000000, 0x2000000), // PLATFORM_BUS
    (0xc000000, virt_plic_size(VIRT_CPUS_MAX)), // PLIC
    (0xc000000, aplic_size(VIRT_CPUS_MAX)),     // APLIC_M
    (0xd000000, aplic_size(VIRT_CPUS_MAX)),     // APLIC_S
    (0x10000000, 0x102000),     // UART and VIRTIO and FW_CFG
    (0x20000000, 0x4000000),    // FLASH
    (0x30000000, 0x10000000),   // PCIE_ECAM
];

/// Memory map list for page level 2 (1GiB per entry).
static VIRT_MEM_MAP_1GB: [(usize, usize); 1] = [
    (1usize << ORDER_1GB, 1usize << ORDER_1GB),     // PCIE_MMIO
];

pub fn get_mem_map_2mb() -> &'static [(usize, usize)] {
    &VIRT_MEM_MAP_2MB
}

pub fn get_mem_map_1gb() -> &'static [(usize, usize)] {
    &VIRT_MEM_MAP_1GB
}
