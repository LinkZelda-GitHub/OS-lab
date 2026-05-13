// typst compile --root . lab-6/main.typ
#import "../base/report.typ": *

#show: report.with(
  title: "实验 0x06 实验报告",
  subtitle: "硬盘驱动与文件系统",
  name: "何亮",
  stdid: "24325005",
  classid: "202520804（陈鹏飞老师班级）",
  major: "计算机科学与技术",
  school: "计算机学院",
  time: "2025 学年第二学期",
  banner: "./images/sysu.png",
)

== MBR 分区表

首先我们需要根据 MBR 的结构定义，利用 `MbrPartition` 的 `parse` 方法来对四个 `partition` 依次解析，如下
```rs
for i in 0..4 {
  let offset = 0x1BE + i * 16;
  partitions.push(
      // FIXED: parse the mbr partition from the buffer
      //      - just ignore other fields for mbr
      MbrPartition::parse(buffer[offset..offset + 16].try_into().unwrap()),
  );
```
然后我们需要利用 `define_filed!` 完成内部数据的解析提取，由于 `CHS` 的 `start` 和 `end` 中的 `sector` 和 `cylinder` 不是占据整个字节的，因此要另外写 `start_sector` 等函数进行解析提取，部分代码如下
```rs
define_field!(u8, 0x00, status); // Status (bit 7 set = active or bootable)
define_field!([u8; 3], 0x01, chs_start); // CHS address of first partition sector (3 bytes)
define_field!(u8, 0x04, partition_type); // Partition type
define_field!([u8; 3], 0x05, chs_end); // CHS address of last partition sector (3 bytes)
define_field!(u32, 0x08, begin_lba); // LBA of partition start
define_field!(u32, 0x0C, total_lba); // Number of sectors in partition

pub fn is_active(&self) -> bool {
    self.status() == 0x80
}

pub fn begin_head(&self) -> u8 {
    self.chs_start()[0]
}

pub fn begin_sector(&self) -> u8 {
    self.chs_start()[1] & 0x3F
}

pub fn begin_cylinder(&self) -> u16 {
    ((self.chs_start()[1] as u16 & 0xC0) << 2) | self.chs_start()[2] as u16
}

// ...... end_head(), end_sector()... is the same as above,
// see entry.rs for the full code
```
然后我们对这部分进行功能测试，需要在根目录的 `Cargo.toml` 的 `member` 中加入 `"crates/storage",` 表项，然后再 `storage/` 下运行 `cargo test`，可以看到，可以通过对应的测试，说明实现大致无误。
#image("./images/MBR_test.png")
