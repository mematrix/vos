代码初始化是跟着[The Adventures of OS: Making a RISC-V Operating System using Rust](https://osblog.stephenmarz.com/index.html)一步步来实现的，由于该系列文章已经过去几年，并且在文章和对应的GitHub仓库中存在一些变动和错误，在此记录部分主要修改。

## ch0-配置环境
按照更新中安装nightly版本并添加RISC-V target，在配置Cargo之后，使用executable编译目标，而不是library，即src目录下为`main.rs`而不是`lib.rs`。这样后面才可以直接用`cargo run`启动（library类型不支持直接用cargo run）。启动命令使用GitHub仓库中`riscv/chapters/ch1`下面的cargo配置。

**关于build**

不再需要原文章中的Makefile+make的方式，统一可以用cargo，这样可以直接在Windows native环境编译+运行。修改内容：将汇编源文件通过`global_asm!`宏引入到Rust文件中混合编译，不再需要独立的GCC额外生成最终elf文件。
