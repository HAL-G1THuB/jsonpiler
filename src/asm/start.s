.section .text.startup,"x"
.p2align 4
.globl _start
.def _start;.scl 2;.type 32;.endef
.seh_proc _start
_start:
  mov rbp, rsp
  .seh_setframe rbp, 0
  sub rsp, 40
  .seh_stackalloc 40
  .seh_endprologue
  .seh_handler .L_SEH_HANDLER, @except
  mov ecx, 65001
  call [qword ptr __imp_SetConsoleCP[rip]]
  test eax, eax
  jz .L_WIN_HANDLER
  mov ecx, 65001
  call [qword ptr __imp_SetConsoleOutputCP[rip]]
  test eax, eax
  jz .L_WIN_HANDLER
  mov ecx, -10
  call [qword ptr __imp_GetStdHandle[rip]]
  cmp rax, -1
  je .L_WIN_HANDLER
  mov qword ptr .L_STDI[rip], rax
  mov ecx, -11
  call [qword ptr __imp_GetStdHandle[rip]]
  cmp rax, -1
  je .L_WIN_HANDLER
  mov qword ptr .L_STDO[rip], rax
  mov ecx, -12
  call [qword ptr __imp_GetStdHandle[rip]]
  cmp rax, -1
  je .L_WIN_HANDLER
  mov qword ptr .L_STDE[rip], rax
