# LAB5

## 实验总结

本实验在 rCore 上实现了多线程与同步原语支持，包括线程的创建与回收（`sys_thread_create`/`waittid`）、自旋锁与阻塞互斥锁、信号量与条件变量，并基于银行家算法实现了死锁检测（`sys_enable_deadlock_detect`）。死锁检测的核心是维护 allocation/need/available 三个矩阵，在每次资源申请阻塞前执行安全性检验，若不存在安全序列则拒绝请求并返回 `-0xdead`。实验加深了对操作系统并发机制的理解，也让我体会到 Rust 借用检查在并发场景下的严格性。

## 简答

1. 在我们的多线程实现中，当主线程 (即 0 号线程) 退出时，视为整个进程退出， 此时需要结束该进程管理的所有线程并回收其资源。 - 需要回收的资源有哪些？ - 其他线程的 TaskControlBlock 可能在哪些位置被引用，分别是否需要回收，为什么？

   **需要回收的资源：**
   - **用户栈**：每个线程在进程地址空间中拥有独立的用户栈（由 `TaskUserRes` 管理），退出时需解除对应的虚拟地址映射并释放物理页帧。
   - **内核栈**：每个线程拥有独立的内核栈（`KernelStack`），其物理帧由内核栈分配器管理，需要释放。
   - **TrapContext 所在页**：内核为每个线程在内核地址空间中单独映射了存放 TrapContext 的页，退出时需解除映射。
   - **线程 ID（tid）**：`TaskUserRes` 析构时会归还 tid 到进程的 `tid_allocator`，需正常回收。
   - **同步原语等待队列中的引用**：若线程正被 Mutex/Semaphore 的等待队列持有，需将其移出以断开 `Arc` 强引用。

   **其他线程 TCB 的引用位置及是否需要主动回收：**

   | 引用位置                                                     | 是否需要主动回收       | 原因                                              |
   | ------------------------------------------------------------ | ---------------------- | ------------------------------------------------- |
   | `ProcessControlBlockInner::tasks[tid]`（`Option<Arc<TCB>>`） | **需要**，置为 `None`  | 这是进程对线程的强引用，不清除会导致 TCB 无法释放 |
   | 调度器就绪队列（`TASK_MANAGER`）                             | **需要**，从队列中移除 | 否则已退出线程仍可能被调度                        |
   | Mutex/Semaphore 阻塞等待队列                                 | **需要**，清除         | 否则 `Arc` 计数不归零，TCB 内存泄漏               |
   | `current_task()`（处理器局部变量）                           | 自动清除               | 线程切走后处理器局部变量会被覆盖，无需手动处理    |

2. 对比以下两种 Mutex 中的实现，二者有什么区别？这些区别可能会导致什么问题？

   **区别：**
   - **`Mutex1::lock`** 用 `loop` 循环：线程被唤醒后会**重新检查** `locked` 是否为 `false`，确认后才设置锁并退出。
   - **`Mutex2::lock`** 不循环：线程从 `block_current_and_run_next()` 返回后**直接继续**，不再检查 `locked`，默认自己已经持锁。
   - **`Mutex1::unlock`** 先将 `locked` 置 `false`，再唤醒等待线程（锁在唤醒瞬间处于空闲状态）。
   - **`Mutex2::unlock`** 若有等待线程则**不清除 `locked`**，直接唤醒（锁的所有权从旧持有者转交给被唤醒线程）。

   **可能导致的问题：**
   - `Mutex1`：`unlock` 时先释放锁再唤醒，此窗口内若另一线程调用 `lock` 并抢先拿到锁，被唤醒线程在 `loop` 中重新检查后会再次入队阻塞，造成**饥饿**风险（公平性差）。但正确性有 `loop` 保障。
   - `Mutex2`：避免了上述抢锁问题（不释放锁直接移交），但 `lock` 中没有重检循环。若出现意外唤醒（spurious wakeup）或内核实现缺陷，线程会**错误地认为自己持有了锁**，破坏互斥性。此外，若等待队列判断出现 bug，`locked` 可能永远不被清除，导致**锁永久占用**。

```rust
impl Mutex for Mutex1 {
 2    fn lock(&self) {
 3        loop {
 4            let mut mutex_inner = self.inner.exclusive_access();
 5            if mutex_inner.locked {
 6                mutex_inner.wait_queue.push_back(current_task().unwrap());
 7                drop(mutex_inner);
 8                block_current_and_run_next();
 9            } else {
10                mutex_inner.locked = true;
11                break;
12            }
13        }
14    }
15
16    fn unlock(&self) {
17        let mut mutex_inner = self.inner.exclusive_access();
18        assert!(mutex_inner.locked);
19        mutex_inner.locked = false;
20        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
21            add_task(waking_task);
22        }
23    }
24}
25
26impl Mutex for Mutex2 {
27    fn lock(&self) {
28        let mut mutex_inner = self.inner.exclusive_access();
29        if mutex_inner.locked {
30            mutex_inner.wait_queue.push_back(current_task().unwrap());
31            drop(mutex_inner);
32            block_current_and_run_next();
33        } else {
34            mutex_inner.locked = true;
35        }
36    }
37
38    fn unlock(&self) {
39        let mut mutex_inner = self.inner.exclusive_access();
40        assert!(mutex_inner.locked);
41        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
42            add_task(waking_task);
43        } else {
44            mutex_inner.locked = false;
45        }
46    }
47}
```

## **荣誉准则**

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > https://rcore-os.cn/rCore-Tutorial-Book-v3/

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
