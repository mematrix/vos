// Defines some macros like the std `print*!`.

#[macro_export]
macro_rules! print_k {
    ($($args:tt)+) => ({
        use core::fmt::Write;
        let _ = write!($crate::driver::uart::Uart::default(), $($args)+);
    });
}

#[macro_export]
macro_rules! println_k {
    () => {
        print_k!("\n")
    };
    ($fmt:expr) => {
        print_k!(concat!($fmt, "\n"))
    };
    ($fmt:expr, $($args:tt)+) => {
        print_k!(concat!($fmt, "\n"), $($args)+)
    };
}
