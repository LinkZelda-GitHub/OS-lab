// typst compile --root . lab-7/main.typ
#import "../base/report.typ": *

#show: report.with(
  title: "实验 0x07 实验报告",
  subtitle: "更好的内存管理",
  name: "何亮",
  stdid: "24325005",
  classid: "202520804（陈鹏飞老师班级）",
  major: "计算机科学与技术",
  school: "计算机学院",
  time: "2025 学年第二学期",
  banner: "./images/sysu.png",
)

== 帧分配器的内存回收

我们在 lab-4 的加分项中已经实现了帧分配器的内存回收，即用一个 `Vec<PhysFrame>` 作为回收的内存池，保存每次释放了的帧，然后每次 allocate 一个新的帧的时候先检查内存池种是否有可用的帧，如下
```rs
unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // FIXED: try pop from recycled frames
        self.used += 1;
        if let Some(reused_frame) = self.recycled.pop() {
            return Some(reused_frame);
        }
        // FIXED: if recycled is empty:
        //        try allocate from frames like before
        self.frames.next()
    }
}

impl FrameDeallocator<Size4KiB> for BootInfoFrameAllocator {
    unsafe fn deallocate_frame(&mut self, _frame: PhysFrame) {
        // FIXED: push frame to recycled
        self.recycled.push(_frame);
        self.used -= 1;
    }
}
```

== 用户程序的内存统计

我们在本实验中只考虑堆内存，栈内存，以及 ELF 文件映射占用的内存，而不考虑页表等占用的内存。而前面两者我们已经实现了，因此我们这部分只需要考虑统计 ELF 文件映射占用的内存。

我们考虑在 `load_elf` 加载 ELF 文件的时候就进行记录，这里根据文档将 `load_elf` 修改为用一个 `Vec<PageRangeInclusive>` 记录所有加载的 ELF 文件映射的页表范围。

随后，我们在 `load_elf_code` 函数中，将返回的 `Vec<PageRangeInclusive>` 进行计算大小并求和，得到该部分占用的内存
```rs
fn load_elf_code(&mut self, elf: &ElfFile, mapper: MapperRef, alloc: FrameAllocatorRef) {
    // FIXED: make the `load_elf` function return the code pages
    self.code =
        elf::load_elf(elf, *PHYSICAL_OFFSET.get().unwrap(), mapper, alloc, true).unwrap();

    // FIXED: calculate code usage
    self.code_usage = self
        .code
        .iter()
        .map(|range| (range.count() as u64) * crate::memory::PAGE_SIZE)
        .sum();
}
```

随后，我们修改 `print_process_list`，加入对内存信息的统计，由于我们实际使用的内存记录在 `used` 成员里面，而且在每次 deallocate 的时候减去了，因此我们这里计算只需要算 `used` 成员即可，如下
```rs
// FIXED: print memory usage of kernel heap
let alloc = get_frame_alloc_for_sure();
let frames_used = alloc.frames_used();
let frames_total = alloc.frames_total();

let used = frames_used * PAGE_SIZE as usize;
let total = frames_total * PAGE_SIZE as usize;

output += &format_usage("Memory", used, total);
drop(alloc);
```
我们可以看到，目前连续运行两次 factorial 的结果如下，
#image("./images/memory_1.png")

这是由于我们还没有实现进程退出时内存的释放，因此此时占用的大小是叠加的，运行两次之后变成了两倍。

== 用户程序的内存释放

首先，我们需要在 `stack.rs` 中实现栈的内存释放函数 `clean_up`，代码如下
```rs
pub fn clean_up(
    &mut self,
    mapper: MapperRef,
    dealloc: &mut impl x86_64::structures::paging::FrameDeallocator<Size4KiB>,
) -> Result<(), x86_64::structures::paging::mapper::UnmapError> {
    let count = self.usage;
    if count == 0 {
        warn!("Stack is empty, no need to clean up.");
        return Ok(());
    }

    // The stack range might have more pages mapped if grown,
    // we unmap the inclusive range.
    let range_inclusive = Page::range_inclusive(self.range.start, self.range.end - 1);
    elf::unmap_range(range_inclusive, mapper, dealloc, true)?;
    self.range = Page::range(self.range.start, self.range.start);

    self.usage = 0;

    Ok(())
}
```
由于我们的内存范围定义是左闭右开的，因此我们要先转化为闭区间 `range_inclusive` 然后调用 `elf::unmap_range()` 进行释放，随后还需要更新栈占用的页面为 0 以及内存的页面范围为一个空区间。

然后我们再在 `proc/vm/mod.rs` 的 `clean_up` 函数中，根据文档加入相关调试信息的输出。

最后，我们需要对 `ProcessVm` 实现 `Drop` trait，调用自己的 `.clean_up()` 函数释放内存，此时 `kill` 函数中调用的 `proc_vm.take()` 在进程退出的时候释放了 `ProcessVm` 的所有权，当 `ProcessVm` 对象没有被任何一个对象持有时，就会自动调用我们实现的 `Drop` trait 中的 `drop` 函数，从而实现了进程退出时的内存释放。

我们可以运行 `factorial`，发现在进程退出之后，使用的内存也被正常释放了！
#image("./images/memory_2.png")

== 内核的内存统计

内核的内存占用包括了内核栈内存的占用以及加载 ELF 文件使用的内存，前者我们已经统计，因此这里只需要实现后者的统计。

我们的方法仍然是在加载内核的 ELF 时就记录下来占用的内存，并记录在 `bootinfo` 中，在初始化内核的时候，把这部分的占用记录到内核的 `ProcessVm` 对象中的 `.code` 和 `.code_usage` 成员中即可。

我们可以观察到，相比于之前的内核占用为 `2.0MiB`，现在变为了 `2.3MiB`，这和 ELF 中 Load 类型的非 .bss 段占用的总内存，即 `0x4b000` Bytes，约为 `0.293MiB` 对应，说明我们成功实现了内核的 ELF 文件的统计！
```
            FileSiz            MemSiz              Flags  Align
LOAD           0x0000000000001000 0xffffff0000000000 0xffffff0000000000
                0x000000000000e084 0x000000000000e084  R      0x1000
LOAD           0x0000000000010000 0xffffff000000f000 0xffffff000000f000
                0x0000000000036bd7 0x0000000000036bd7  R E    0x1000
LOAD           0x0000000000047000 0xffffff0000046000 0xffffff0000046000
                0x0000000000004148 0x0000000000004148  RW     0x1000
```
#image("./images/memory_3.png")

== 内核栈的自动增长

从上面的部分来看，我们的内核有 `2.0MiB` 的内存占用来自内核栈，这是因为我们为了避免内核栈溢出而分配了大量的内存，因此这部分我们希望把内核栈改为自动增长而不是预先分配大量空间。

这里利用的是 config 里面的 `kernel_stack_auto_grow` 配置项，如果是 `0` 则认为一次性分配了内核栈的所有内存，如果非 0，则表示初始时分配给内核栈的内存。

然后在进行 `map_range` 的时候，需要重新计算需要映射的栈大小以及当前栈底地址，这个可以通过将内核栈最低地址加上未映射的部分得到，代码参考文档给出的进行实现，请见 `boot/src/main.rs`。

随后，我们在 `exception.rs` 中对 `USER_MODE` 位进行判断，并输出对应的日志，同时根据文档给出的代码，在 `lib.rs` 中加入对内核栈缺页异常处理的测试。

可以看到，可以正常通过内核栈增长的测试，并且此时我们 `kernel` 占用的 memory 变成了 `340.0KiB` 说明，我们对内核栈自动扩容的实现符合预期！
#image("./images/memory_4.png")

== 用户态堆的实现

== 思考题

=== 当在 Linux 中运行程序的时候删除程序在文件系统中对应的文件，会发生什么？程序能否继续运行？遇到未被映射的内存会发生什么？

=== 为什么要通过 `Arc::strong_count` 来获取 `Arc` 的引用计数？查看它的定义，它和一般使用 `&self` 的方法有什么不同？出于什么考虑不能直接通过 `&self` 来进行这一操作？

=== `bootloader` 加载内核并为其分配初始栈区时，至少需要多少页内存才能保证内核正常运行？
- 尝试逐渐增大内核的栈区大小，观察内核的运行情况，对于不能正常启动的情况，尝试分析可能的原因。
- 提示：内核实现缺页中断的处理时，依赖于哪些子系统？报错是什么？什么子系统可能会导致对应的问题？

经过测试，至少需要 4 页内存才能保障内核的正常运行，当初始栈区设为 3 页时，会在如下输出后重启，
#image("./images/test_3.png")

设为 2 页及以下的时候，会在如下输出后重启，
#image("./images/test_2.png")

上述两者都是由于中断处理未初始化，从而引发了 triple fault，前者是因为我们的 IDT 需要占用一定的空间因此页数不够时会 page fault，后者是因为在 `Frame Allocator` 初始化的时候，其中的 `create_frame_inter` 函数中使用了 `.clone`，此处是将其进行深拷贝放入当前函数的栈，即内核栈中，从而引发了栈溢出 page fault。

其次，由于缺页中断处理依赖于中断子系统以及内存子系统，因此在 `Interrupts Initialized.` 之前和缺页异常栈 `Page Fault IST` 初始化之前，缺页中断都不能正常对内核栈进行扩容，缺页异常由于找不到对应的中断处理程序或者没有内核栈，从而再次抛出异常，引发 triple fault 导致操作系统没有任何提示而直接不断重启。

=== 尝试查找资料，了解 `mmap`、`munmap` 和 `mprotect` 系统调用的功能和用法，回答下列问题：

- `mmap` 的主要功能是什么？它可以实现哪些常见的内存管理操作？

- `munmap` 的主要功能是什么？什么时候需要使用 `munmap`？

- `mprotect` 的主要功能是什么？使用 `mprotect` 可以实现哪些内存保护操作？

- 编写 C 程序，使用 mmap 将一个文件映射到内存中，并读写该文件的内容。

- 思考：文件内容什么时候会被写入到磁盘？

- 综合考虑有关内存、文件、I/O 等方面的知识，讨论为什么 `mmap` 系统调用在现代操作系统中越来越受欢迎，它具有哪些优势？
