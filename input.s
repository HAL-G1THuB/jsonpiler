.global start
.extern WriteConsoleW
.extern GetLastError
.extern SetConsoleCP, SetConsoleOutputCP
.extern ExitProcess
.extern FormatMessageW
.extern MessageBoxA
.extern GetStdHandle
.section .data
  l_01: .string "345"
  l_02: .string "$"
  l_04: .string "title"
  l_05: .string "345"
.section .bss
  .lcomm lastError, 4
  .lcomm errorMessage, 512
  .lcomm STDOUT, 8
  .lcomm STDERR, 8
  .lcomm STDIN, 8
  .lcomm l_03, 8
  .lcomm l_06, 8
.section .text
_start:
  subq $40, %rsp
  movl $65001, %ecx
  callq SetConsoleCP
  movl $65001, %ecx
  callq SetConsoleOutputCP
  movl $-10, %ecx
  callq GetStdHandle
  movq %rax, STDIN(%rip)
  movl $-11, %ecx
  callq GetStdHandle
  movq %rax, STDOUT(%rip)
  movl $-12, %ecx
  callq GetStdHandle
  movq %rax, STDERR(%rip)
  xorl %ecx, %ecx
  leaq l_01(%rip), %rdx
  leaq l_02(%rip), %r8
  xorl %r9d, %r9d
  callq MessageBoxA
  testl %eax, %eax
  jz display_error
  movq %rax, l_03(%rip)

  xorl %ecx, %ecx
  leaq l_05(%rip), %rdx
  leaq l_04(%rip), %r8
  xorl %r9d, %r9d
  callq MessageBoxA
  testl %eax, %eax
  jz display_error
  movq %rax, l_06(%rip)

  xorl %ecx, %ecx
  callq ExitProcess
display_error:
  callq GetLastError
  movl %eax, lastError(%rip)
  subq $32, %rsp
  movl $0x1200, %ecx
  xorl %edx, %edx
  movl lastError(%rip), %r8d
  xorl %r9d, %r9d
  leaq errorMessage(%rip), %rax
  movq %rax, 32(%rsp)
  movl $1024, 40(%rsp)
  movq $0, 48(%rsp)
  callq FormatMessageW
  addq $16, %rsp
  testl %eax, %eax
  jz exit_program
  movq STDERR(%rip), %rcx
  leaq errorMessage(%rip), %rdx
  movq $256, %r8
  leaq 32(%rsp), %r9
  movq $0, 40(%rsp)
  addq $16, %rsp
  callq WriteConsoleW
exit_program:
  movl lastError(%rip), %ecx
  callq ExitProcess
