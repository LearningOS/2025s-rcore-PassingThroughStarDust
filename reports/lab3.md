（注意： 实验手册里把 stride scheduling 中的 stride 和 pass 的含义搞反了，本文全部按纠正后的含义表述）

# 1. 简单总结你实现的功能（200字以内，不要贴代码）。
1. 实现 sys_spawn
    > a. 拆解 sys_fork 与 sys_exec 的实现，去除了拷贝父进程 memory_set 的步骤。
2. 实现 sys_set_priority
    > a. 在 TaskControlBlockInner 中添加 priority 、 stride 、pass 3 个成员变量（按约定，所有可变成员变量都放在 TaskControlBlockInner 中），并按照约定在 TaskControlBlock 的 new 、 fork 和 spawn 中对它们进行初始化。<br>
      b. 为 TaskControlBlock 增加 set_priority 成员函数修改 priority 值，并按照 stride scheduling 定义同步变化

# 2. 完成问答题。
1. stride 算法深入
    1) 不能， u8 类型所能表达的数字上限为 255 ， p2 执行完时间片后 pass += stride 会导致数据溢出， 得到 pass == 5 ， 导致下一次又是 p2 被执行。
    2) 由 stride 的计算公式 $$ stride = \frac{BigStride}{priority} $$ 得出，stride 的值域为 $$ stride \in [1 , \frac{BigStride}{2}] $$ (左侧不能为0， 不然 pass 的值不会增加，导致该进程被永恒执行)， 因此可以得到 $$ \max{(STRIDE_ -MAX – STRIDE_ -MIN)} = \frac{BigStride}{2} - 1 \leq \frac{BigStride}{2} $$ 证明完毕。
    3) 代码实现如下：
    ```rust
    use core::cmp::Ordering;

    struct Pass(u64);

    impl PartialOrd for Pass {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            let diff = self.0 - other.0;
            if (diff.abs() << 1) < BIG_STRIDE {
                if d < 0 { Some(Ordering::Greater) } else { Some(Ordering::Less) }
            } else {
                if d < 0 { Some(Ordering::Less) } else { Some(Ordering::Greater) }
            }
            /* 
                Another method based on the fact that Pass is integer multiple of Stride (but this requires in).

                But a potential bug could be inevitable if only apply comparing method on Pass: if Pass == (n * 10 + t) * stride (n is any usigned integer while t < 10), then it has the same performance with Pass that is equal to t * stride (t < 10). To solve it, renewing Pass value when n == 10 is effective.
            */
            
        }
    }

    impl PartialEq for Pass {
        fn eq(&self, other: &Self) -> bool {
            false
        }
    }
    ```

# 3. 加入荣誉准则的内容。否则，你的提交将视作无效，本次实验的成绩将按“0”分计。
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

（无）

此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

（无）

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

# 4. (optional) 你对本次实验设计及难度/工作量的看法，以及有哪些需要改进的地方，欢迎畅所欲言。
无。