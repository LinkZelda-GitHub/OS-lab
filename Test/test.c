#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>
#include <string.h>

int main() {
    const char *filepath = "test.txt";

    // O_RDWR: 以读写方式打开文件
    // O_CREAT: 如果文件不存在则创建文件
    // 0666: 创建文件的权限，表示所有用户都具有读写权限
    int fd = open(filepath, O_RDWR | O_CREAT, 0666);
    
    // 设置文件大小为 100 bytes
    ftruncate(fd, 100);
    
    // 将文件映射到内存
    // PROT_READ | PROT_WRITE: 映射区域可读可写
    // MAP_SHARED: 映射区域的修改会反映到文件中
    // 也可也设置为 MAP_PRIVATE: 映射区域的修改不会反映到文件中，相当于 COW 机制
    // NULL: 由系统选择映射地址
    // 0: 从文件的开头开始映射
    // 返回映射的地址，如果失败则返回 MAP_FAILED
    char *mapped = mmap(NULL, 100, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (mapped == MAP_FAILED) {
        perror("mmap failed");
        return 1;
    }
    
    // 写文件内容
    strcpy(mapped, "Hello from mmap!");
    
    // 读文件内容
    printf("Read from memory: %s\n", mapped);
    
    // 取消映射与关闭文件
    munmap(mapped, 100);
    close(fd);
    return 0;
}