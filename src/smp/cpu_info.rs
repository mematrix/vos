//! CPU information.


/// Represents the CPU info.
#[repr(C)]
pub struct CpuInfo {
    /// CPU frequency.
    clock_freq: usize,
    /// CPU timebase frequency. This is the frequency of the RTC (Realtime Clock), we can use
    /// the value to compute the clock tick count that a time slice need. For example, a time
    /// slice of *10ms* have the tick count of `timebase_freq / 1000 * 10`.
    ///
    /// The `mtime` register value will increase at this frequency.
    timebase_freq: usize,
    /// Cache the hart id, because the `mhartid` is a machine level CSR and we need the env-call
    /// to get the hart-id.
    hart_id: usize,
    /// A quick reference to get the cpu_id of current `CpuInfo` object.
    cpu_id: usize,
    // Extensions supported by the CPU.
    //extensions: usize,
}

/// Normal process time slice that a second is divided. Currently we set it to 128 (equals to a
/// `>> 7` ops) so the normal process' time slice is `~8ms`.
const TIME_SLICE_OF_NORMAL: usize = 128usize;
/// Realtime process time slice that a second is divided. Currently we set it to 256 (equals to
/// a `>> 8` ops) so the realtime process' time slice is `~4ms`.
const TIME_SLICE_OF_REALTIME: usize = 256usize;

impl CpuInfo {
    // We construct the `Cpu` object by performing a C-style cast from ptr instead of the usual
    // constructor call, so no ctor method is provided.

    #[inline(always)]
    pub fn set_clock_freq(&mut self, freq: usize) {
        self.clock_freq = freq;
    }

    #[inline(always)]
    pub fn get_clock_freq(&self) -> usize {
        self.clock_freq
    }

    #[inline(always)]
    pub fn set_timebase_freq(&mut self, freq: usize) {
        self.timebase_freq = freq;
    }

    #[inline(always)]
    pub fn get_timebase_freq(&self) -> usize {
        self.timebase_freq
    }

    /// Get the interval time (in CPU clocks) performing the context switching.
    #[inline(always)]
    pub fn get_ctx_switch_interval(&self) -> usize {
        self.timebase_freq / 64usize
    }

    #[inline(always)]
    pub fn set_hart_id(&mut self, hard_id: usize) {
        self.hart_id = hard_id;
    }

    #[inline(always)]
    pub fn get_hart_id(&self) -> usize {
        self.hart_id
    }

    #[inline(always)]
    pub fn set_cpu_id(&mut self, cpu_id: usize) {
        self.cpu_id = cpu_id;
    }

    #[inline(always)]
    pub fn get_cpu_id(&self) -> usize {
        self.cpu_id
    }
}
