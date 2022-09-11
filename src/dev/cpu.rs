//! CPU and CPU-related routines. Provides the operations with the CSRs.
//!
//! **Note**: After the `/driver/cpu` mod has been initialized, this mod becomes available.

/// Represents the CPU info.
pub struct Cpu {
    /// CPU frequency. We will perform the context switching per 10ms (100 times per second),
    /// so the context switch time is `freq / 100`.
    freq: u64,
    /// Cache the hart id, because the `mhartid` is a machine level CSR and we need the env-call
    /// to get the hart-id.
    hart_id: usize,
    // Extensions supported by the CPU.
    //extensions: usize,
}

impl Cpu {
    // We don't construct the `Cpu` object by performing a C-style cast instead of the usual
    // constructor call, so no ctor method is provided.

    #[inline(always)]
    pub fn set_freq(&mut self, freq: u64) {
        self.freq = freq;
    }

    #[inline(always)]
    pub fn get_freq(&self) -> u64 {
        self.freq
    }

    /// Get the interval time (in CPU clocks) performing the context switching.
    #[inline(always)]
    pub fn get_ctx_switch_interval(&self) -> u64 {
        // or freq/128 ?
        self.freq / 100
    }

    #[inline(always)]
    pub fn set_hart_id(&mut self, hard_id: usize) {
        self.hart_id = hard_id;
    }

    #[inline(always)]
    pub fn get_hart_id(&self) -> usize {
        self.hart_id
    }
}


#[repr(usize)]
pub enum Register {
    Zero = 0,
    Ra,
    Sp,
    Gp,
    Tp,
    T0,
    T1,
    T2,
    S0,
    S1,
    A0, /* 10 */
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    S2,
    S3,
    S4, /* 20 */
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,
    T3,
    T4,
    T5, /* 30 */
    T6
}

pub const fn reg(r: Register) -> usize {
    r as usize
}

// Floating point registers
#[repr(usize)]
pub enum FRegister {
    Ft0,
    Ft1,
    Ft2,
    Ft3,
    Ft4,
    Ft5,
    Ft6,
    Ft7,
    Fs0,
    Fs1,
    Fa0, /* 10 */
    Fa1,
    Fa2,
    Fa3,
    Fa4,
    Fa5,
    Fa6,
    Fa7,
    Fs2,
    Fs3,
    Fs4, /* 20 */
    Fs5,
    Fs6,
    Fs7,
    Fs8,
    Fs9,
    Fs10,
    Fs11,
    Ft8,
    Ft9,
    Ft10, /* 30 */
    Ft11
}

pub const fn freg(r: FRegister) -> usize {
    r as usize
}
