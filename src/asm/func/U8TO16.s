.L_U8TO16:
	push	rdi
	push	rsi
	push	rbx
  push rbp
  mov rbp, rsp
  sub rsp, 48
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
