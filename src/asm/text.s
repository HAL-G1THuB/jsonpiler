.L_WIN_HANDLER:
  call [qword ptr __imp_GetLastError[rip]]
  mov ebx, eax
  sub rsp, 32
  mov ecx, 0x1300
  xor edx, edx
  mov r8d, ebx
  xor r9d, r9d
  lea rax, qword ptr .L_WIN_HANDLER_MSG[rip]
  mov qword ptr 0x20[rsp], rax
  mov dword ptr 0x28[rsp], 0
  mov dword ptr 0x30[rsp], 0
  call [qword ptr __imp_FormatMessageW[rip]]
  test eax, eax
  jz .L_EXIT
  xor ecx, ecx
  mov rdx, qword ptr .L_WIN_HANDLER_MSG[rip]
  xor r8d, r8d
  mov r9d, 0x10
  call [qword ptr __imp_MessageBoxW[rip]]
.L_EXIT:
  mov rcx, qword ptr .L_WIN_HANDLER_MSG[rip]
  call [qword ptr __imp_LocalFree[rip]]
  mov ecx, ebx
  call [qword ptr __imp_ExitProcess[rip]]
.L_SEH_HANDLER:
  sub rsp, 40
  xor ecx, ecx
  lea rdx, .L_SEH_HANDLER_MSG[rip]
  xor r8d, r8d
  mov r9d, 0x10
  call [qword ptr __imp_MessageBoxA[rip]]
  mov ecx, -1
  call [qword ptr __imp_ExitProcess[rip]]
.section .text$U8TO16, "x"
.seh_proc .L_U8TO16
.L_U8TO16:
	push	rdi
	.seh_pushreg	rdi
	push	rsi
	.seh_pushreg	rsi
	push	rbx
	.seh_pushreg	rbx
  push rbp
  .seh_pushreg rbp
  mov rbp, rsp
  .seh_setframe rbp, 0
  sub rsp, 48
  .seh_stackalloc 48
  .seh_endprologue
  .seh_handler .L_SEH_HANDLER, @except
  mov rdi, rcx
  mov ecx, 65001
  xor edx, edx
  mov r8, rdi
  mov r9d, -1
  mov qword ptr 0x20[rsp], 0
  mov qword ptr 0x28[rsp], 0
  call [qword ptr __imp_MultiByteToWideChar[rip]]
  test eax, eax
  jz .L_WIN_HANDLER
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
  jz .L_WIN_HANDLER
  mov rax, rbx
  mov rsp, rbp
  pop rbp
	pop	rbx
	pop	rsi
	pop	rdi
  ret
.seh_endproc
