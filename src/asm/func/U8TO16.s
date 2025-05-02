  .seh_proc .L__U8TO16
.L__U8TO16:
  push rdi
  .seh_pushreg rdi
  push rsi
  .seh_pushreg rsi
  push rbx
  .seh_pushreg rbx
  sub rsp, 0x38
  .seh_stackalloc 0x38
  .seh_endprologue
  .seh_handler .L__SEH_HANDLER, @except
  mov rdi, rcx
  mov ecx, 65001
  xor edx, edx
  mov r8, rdi
  mov r9d, -1
  mov qword ptr 0x20[rsp], 0
  mov qword ptr 0x28[rsp], 0
  call [qword ptr __imp_MultiByteToWideChar[rip]]
  test eax, eax
  jz .L__WIN_HANDLER
  shl rax, 1
  mov rsi, rax
  mov rcx, rsi
  call [qword ptr __imp_malloc[rip]]
  mov rbx, rax
  mov ecx, 65001
  xor edx, edx
  mov r8, rdi
  mov r9d, -1
  mov qword ptr 0x20[rsp], rbx
  mov qword ptr 0x28[rsp], rsi
  call [qword ptr __imp_MultiByteToWideChar[rip]]
  test eax, eax
  jz .L__WIN_HANDLER
  mov rax, rbx
  add rsp, 0x38
  pop rbx
  pop rsi
  pop rdi
  ret
.seh_endproc
