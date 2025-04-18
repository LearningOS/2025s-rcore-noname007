## 1
> 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

> 越权执行指令，陷入到 supervisor 态，在 trap_handle 中进行具体的处理

### 输出结果
```shell
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
```

### trap_handle 处理逻辑
```rust
Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
    println!("[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
    exit_current_and_run_next();
}
Trap::Exception(Exception::IllegalInstruction) => {
    println!("[kernel] IllegalInstruction in application, kernel killed it.");
    exit_current_and_run_next();
}
```

## 2
深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:
### 2.1
>L40：刚进入 __restore 时，sp 代表了什么值。请指出 __restore 的两种使用情景。
1. 内核栈指针
2. 第一次运行应用程序，从内核态返回用户态

### 2.2
> L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

```asm
ld t0, 32*8(sp)
ld t1, 33*8(sp)
ld t2, 2*8(sp)
csrw sstatus, t0
csrw sepc, t1
csrw sscratch, t2
```
| 寄存器      | 作用                                               |
|----------|--------------------------------------------------|
| sstatus  | 执行 sret指令时，根据 sstatus 的 SPP 字段设置 CPU 特权级为 U 或者 S |
| sepc     | 执行 sret指令时， 会将其值赋值给 PC 寄存器                       |
| sscratch | 陷入之前任务的栈指针                                   |

### 2.3 L50-L56：为何跳过了 x2 和 x4？
```asm
ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
LOAD_GP %n
.set n, n+1
.endr
```
x2 是 sp 的别名，如果此时设置，会影响后续其他寄存器值的恢复

x4 一般情况下不会使用到，可以忽略
### 2.4 
> L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？
```asm
csrrw sp, sscratch, sp
```
对 sp，sscratch 寄存器中的值执行原子指令交换

### 2.5
> __restore：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

`sret` 会设置相关的寄存器，及根据 sstatus 恢复CPU 的特权级

### 2.6
> L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？

```asm
csrrw sp, sscratch, sp
```
栈切换
sp 从指向用户栈，改为指向内核栈
sscratch 保存用户栈指针值

### 2.7
> 从 U 态进入 S 态是哪一条指令发生的？

`ecall`