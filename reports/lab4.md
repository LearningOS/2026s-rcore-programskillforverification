# LAB4

## 实验总结

本次实验实现了文件系统相关的三个系统调用。`sys_linkat` 在根目录末尾追加一条指向已有 inode_id 的 `DirEntry`，创建硬链接；`sys_unlinkat` 将对应目录项用空 `DirEntry` 覆盖实现逻辑删除；`sys_fstat` 通过 `block_id/block_offset` 反算 inode_id，扫描根目录统计 nlink，填充 `Stat` 结构写回用户态。为此在 easy-fs 的 `Inode` 上新增了 `get_inode_id`、`link_count`、`link`、`unlink` 方法，并在 `File` trait 增加了默认 `fstat` 方法。

## 简答

1. 在我们的easy-fs中，root inode起着什么作用？如果root inode中的内容损坏了，会发生什么？

   **root inode 的作用**：easy-fs 是一个单目录（flat）文件系统，根目录的 inode（inode_id = 0）是整个文件系统唯一的目录节点，其数据区存储所有文件的 `DirEntry`（name → inode_id 的映射表）。所有文件的创建（`create`）、查找（`find`）、链接（`link`）、解链（`unlink`）和列目录（`ls`）操作都必须经过根目录 inode 来定位目标 inode_id，再通过 `EasyFileSystem::get_disk_inode_pos(inode_id)` 换算出磁盘块位置读写实际 `DiskInode`。

   **损坏后的影响**：root inode 的 `DiskInode.size` 字段决定目录条目数量，`direct/indirect` 块指针决定实际数据在磁盘上的位置。若 root inode 损坏：
   - `size` 错误会导致 `find`/`ls` 读到垃圾数据，返回错误的文件名或 inode_id，进而打开错误的文件内容甚至越界读写磁盘块。
   - 块指针损坏会使所有文件查找直接失败（找不到任何 `DirEntry`），系统无法打开、创建或删除任何文件，相当于文件系统完全不可用。
   - 由于 easy-fs 没有日志、备份或 fsck 机制，root inode 损坏后无法自动修复，只能重新格式化文件系统（丢失所有数据）。

## **荣誉准则**

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > https://rcore-os.cn/rCore-Tutorial-Book-v3/

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
