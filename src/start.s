.text
.globl _start
.def	_start; .scl	2; .type	32; .endef
.seh_proc _start
_start:
  sub rsp, 40
  .seh_stackalloc 40
  .seh_endprologue
  .seh_handler exception_handler, @except
  mov ecx, 65001
  call SetConsoleCP
  test rax, rax
  jz display_error
  mov ecx, 65001
  call SetConsoleOutputCP
  test rax, rax
  jz display_error
  mov ecx, -10
  call GetStdHandle
  cmp rax, -1
  je display_error
  mov QWORD PTR STDI[rip], rax
  mov ecx, -11
  call GetStdHandle
  cmp rax, -1
  je display_error
  mov QWORD PTR STDO[rip], rax
  mov ecx, -12
  call GetStdHandle
  cmp rax, -1
  je display_error
  mov QWORD PTR STDE[rip], rax
