# 1. 简单总结你实现的功能（200字以内，不要贴代码）。
在 os/src/syscall/process.rs 中的 sys_trace 函数中通过 match 匹配实现了对不同 _trace_request 值的匹配，
当 _trace_request 的值为：
    > 0, 将 _id 转换为 *const u8 ，并调用转换后的对象中的 read_volatile 方法读取一个字节并转换返回。
    > 1, 将 _id 转换为 *mut u8 ，并调用转换后的对象中的 write_volatile 方法写入 _data 的低8位，最后返回数值0。
    > 2, 将 _id 作为 syscall id ，调用我自己实现的函数 get_syscall_count 获取当前应用调用对应 syscall 的次数统计。

* get_syscall_count 的实现
预备条件：
    > 将 os/src/syscall/mod.rs 中的所有 syscall id 值均声明为pub。
    > 在 os/src/config.rs 中声明常数 NUM_SYSCALL_TYPE 表示当前所有系统调用类型的个数。
功能实现：
    1. 在 os/src/task/mod.rs 中，扩展 TaskManagerInner ，添加一个二维数组 syscall_count 用于统计当前不同应用调用各个 syscall 的次数。
    2. 为 TaskManager 实现两个方法 add_syscall_count 和 get_syscall_count ，分别用于增加当前应用调用对应 syscall 的次数和获得调用对应
    syscall 的次数。在外部将他们分装成独立的 pub 函数 add_syscall_count 和 get_syscall_count。
    3. 在 os/src/syscall/mod.rs 的 syscall 函数的开头添加 add_syscall_count ，最后在 os/src/syscall/process.rs 的 sys_trace 中
    调用 get_syscall_count 即可获取目标 syscall 的调用次数。

# 2. 完成问答题。
1. (使用的 SBI 版本为 RustSBI-QEMU version 0.1.1 ) 对 bad 测例：
    > ch2b_bad_address ，运行返回异常 StoreFault ，原因是试图向地址 0x0（ NULL 值的保留地址）写入字节（非法操作）。
    > ch2b_bad_instructions ，运行返回异常 IllegalInstruction ，原因是试图在 U mode 执行 S mode 特权指令 sret。
    > ch2b_bad_register ，运行返回异常 IllegalInstruction ，原因是试图在 U mode 访问 S mode 的 CSR sstatus。
2. 
    1) sp 代表了对应应用的内核栈栈顶。
    2) 特殊处理了： 
        > sstatus ，其中的 SPP 等字段会给出 Trap 发生前CPU 处在哪个特权级（S/U）等信息。该寄存器的值决定了 trap 处理完成后 CPU 能否返回正确的特权级。
        > sepc ，当 Trap 是一个异常的时候，记录 Trap 发生之前执行的最后一条指令的地址。当 Trap 是一个异常的时候，该寄存器记录 Trap 发生之前执行的最后一条指令的地址。
        > sscratch ，在最开始保存了内核栈的地址，并在处理 Trap 时可作为中介暂时保存 sp（ sp 指向用户栈或内核栈）的地址。该寄存器在处理 trap的过程中有效地辅助了用户栈道和内核栈的切换。
    3) 原因分别如下：
        > x2 是 sp ，保存通用寄存器的值的过程依赖于基于 sp 的偏移位置，而 sp 能够在每次调用 __restore 时都存储着正确的值是通过该段取出通用寄存器值的程序之前的代码实现的。将 sp 压入栈没有意义，因为只要前文代码正确，每次到 __restore 执行时取出的 sp 值必定会等于当前 sp 的值。
        > x4 是 tp ，除非我们手动出于一些特殊用途使用它，否则一般也不会被用到。
    4) sp 指向用户栈，ssratch 指向内核栈。
    5) sret ，sret 被调用后，CPU 会根据 sstatus 中的 SPP 字段将特权级设置为 S/U ，在本程序中SPP字段已经通过代码的方式保证为 U mode。
    6) sp 指向内核栈，ssratch 指向内核栈。
    7) ecall 指令（位于user/src/syscall.rs 中的几个内联汇编代码中包含"ecall" 指令的函数）。

# 3. 加入荣誉准则的内容。否则，你的提交将视作无效，本次实验的成绩将按“0”分计。
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

（我在完成本次实验的过程中无相关交流对象，与他人的交流是在学习 rCore-Tutorial-v3 中的内容时遇到的对代码的疑惑以及咨询相关工具、环境的搭建方法）

此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

（在完成本次实验的过程中我未查阅与完成实验内容相关的资料）

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

# 4. (optional) 你对本次实验设计及难度/工作量的看法，以及有哪些需要改进的地方，欢迎畅所欲言。
本次实验难度我觉得适中，最大的障碍在于读懂整个项目的代码，在读懂代码后就能较为容易地实现。