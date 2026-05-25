#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>

int main() {
    int pid = getpid();
    int count = 0;
    int *invalid_ptr = NULL; // 未映射的内存地址（空指针）

    printf("【程序已启动】PID 是: %d\n", pid);
    printf("请在另一个终端运行: rm ./test\n\n");

    while (1) {
        printf("[%d] 程序仍在运行... 磁盘文件可能已经被删除了。\n", count++);
        sleep(1);

        // 运行到第 15 秒时，故意踩向未映射内存
        if (count == 15) {
            printf("\n⚠️ 准备访问未映射的内存 (引发段错误)...\n");
            *invalid_ptr = 42; // 解引用空指针，触发 SIGSEGV
        }
    }
    return 0;
}