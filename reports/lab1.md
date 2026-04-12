# LAB1

## 实验总结

本次实现了 `sys_trace` 的三种功能：`trace_request=0` 按 `*const u8` 读取用户地址一个字节并返回；`trace_request=1` 按 `*mut u8` 将 `data` 低 8 位写回用户地址并返回 0；`trace_request=2` 查询当前任务指定系统调用号的累计调用次数（含本次调用）。同时在系统调用总入口统一记录每次调用，并对非法请求或越界编号返回 -1，保证行为可预测与安全。

## 简答

1. 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (``ch2b_bad_*.rs`)）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

RustSBI: 0.3.0-alpha.2
Platform implementation (RustSBI-QEMU): 0.2.0-alpha.2

- `ch2b_bad_address`: 在用户态直接向地址 0x0 写一个字节。这是非法地址访问 - `[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003c4, kernel killed it.`
- `ch2b_bad_instructions`: 在用户态直接执行 sret。sret 属于特权指令，U 模式不能执行 -`[kernel] IllegalInstruction in application, kernel killed it.`
- `ch2b_bad_register`: 在用户态用 csrr 读取 sstatus。sstatus 是特权级寄存器，用户态无权访问 - `[kernel] IllegalInstruction in application, kernel killed it.`

2. 深入理解 `trap.S` 中两个函数 `__alltraps` 和 `__restore` 的作用，并回答如下问题:

（1）. L40：刚进入 `__restore` 时，`sp` 代表了什么值。请指出 `__restore` 的两种使用情景。
刚进入 `__restore` 时，`sp` 指向当前任务内核栈上的 TrapContext 起始地址（即保存好的陷入现场）；`sscratch` 中保存的是用户栈指针。

`__restore` 的两种使用情景：

- Trap 返回：用户态触发异常/系统调用后进入 `__alltraps`，内核处理完毕从 `trap_handler` 返回，随后执行 `__restore`，恢复寄存器并通过 `sret` 返回用户态。
- 首次运行任务/任务切换后恢复：调度器通过 `__switch` 切换到某任务，其 `TaskContext.ra` 预置为 `__restore`，`TaskContext.sp` 预置为该任务 TrapContext 地址；因此 `ret` 后直接进入 `__restore` 恢复该任务并进入用户态。

（2）. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。
这几行特殊处理了 `sstatus`、`sepc` 和 `sscratch`：

- `sstatus`：恢复特权与中断相关状态，其中 `SPP` 决定 `sret` 后返回到 U/S 态。
- `sepc`：恢复返回地址，`sret` 后 PC 跳到 `sepc` 指向的用户指令继续执行。
- `sscratch`：恢复用户栈指针备份，配合 trap 入口/返回时与 `sp` 交换，保证内核栈与用户栈切换正确。

（3）. L50-L56：为何跳过了 `x2` 和 `x4`？
`x2` 是 `sp`，不按通用寄存器方式恢复，而是在末尾通过 `csrrw sp, sscratch, sp` 与 `sscratch` 交换来恢复用户栈；`x4` 是 `tp`，本实验用户程序不使用线程本地指针，因此不必保存/恢复。

（4）. L60：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？
L60 是 `csrrw sp, sscratch, sp`。执行后：

- `sp` 变为用户栈顶（即将以用户栈继续执行）。
- `sscratch` 变为内核栈顶（下次 trap 进入时可直接换回内核栈）。

（5）. `__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？
发生在 `sret`。其原因是 trap 返回时硬件根据 `sstatus.SPP` 决定返回特权级；此前已恢复 `sstatus` 且 `SPP=U`，并且 `sepc` 已设为用户态下一条指令地址，因此 `sret` 后切换到 U 态执行。

（6）. L13：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？
L13 是 trap 入口处 `csrrw sp, sscratch, sp`。执行后：

- `sp` 从用户栈切换为内核栈（用于保存 TrapContext）。
- `sscratch` 保存原用户栈指针（供返回用户态时恢复）。

（7）. 从 U 态进入 S 态是哪一条指令发生的？
从 U 态进入 S 态由触发异常/中断的指令引发（典型是用户程序执行 `ecall`）；硬件完成陷入后转到 `stvec` 指向的 `__alltraps`。因此“触发切换”的是用户侧 `ecall`（或其他异常指令），而内核侧第一条可见指令是 `__alltraps` 的 `csrrw sp, sscratch, sp`。

## **荣誉准则**

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > https://rcore-os.cn/rCore-Tutorial-Book-v3/

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
