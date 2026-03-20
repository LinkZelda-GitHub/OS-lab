// typst compile --root . lab-2/main.typ
#import "../base/report.typ": *

#show: report.with(
  title: "实验 0x02 实验报告",
  subtitle: "中断处理",
  name: "何亮",
  stdid: "24325005",
  classid: "202520804（陈鹏飞老师班级）",
  major: "计算机科学与技术",
  school: "计算机学院",
  time: "2025 学年第二学期",
  banner: "./images/sysu.png"
)

== GDT 与 TSS

