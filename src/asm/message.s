  sub rsp, 16
  mov ecx, 65001
  xor edx, edx
  lea r8, qword ptr {}[rip]
  mov r9d, -1
  mov qword ptr 0x20[rsp], 0
  mov qword ptr 0x28[rsp], 0
  call [qword ptr __imp_MultiByteToWideChar[rip]]
  test rax, rax
  jz .L_WIN_HANDLER
  shl rax, 1
  mov rdi, rax
  mov rcx, rax
  call [qword ptr __imp_malloc[rip]]
  mov r12, rax
  mov ecx, 65001
  xor edx, edx
  lea r8, qword ptr {}[rip]
  mov r9d, -1
  mov qword ptr 0x20[rsp], r12
  mov qword ptr 0x28[rsp], rdi
  call [qword ptr __imp_MultiByteToWideChar[rip]]
  test rax, rax
  jz .L_WIN_HANDLER
  mov qword ptr {}[rip], r12
  mov ecx, 65001
  xor edx, edx
  lea r8, qword ptr {}[rip]
  mov r9, -1
  mov qword ptr 0x20[rsp], 0
  mov qword ptr 0x28[rsp], 0
  call [qword ptr __imp_MultiByteToWideChar[rip]]
  test rax, rax
  jz .L_WIN_HANDLER
  shl rax, 1
  mov rdi, rax
  mov rcx, rax
  call [qword ptr __imp_malloc[rip]]
  mov r12, rax
  mov ecx, 65001
  xor edx, edx
  lea r8, qword ptr {}[rip]
  mov r9, -1
  mov qword ptr 0x20[rsp], r12
  mov qword ptr 0x28[rsp], rdi
  call [qword ptr __imp_MultiByteToWideChar[rip]]
  test rax, rax
  jz .L_WIN_HANDLER
  mov qword ptr {}[rip], r12
  xor ecx, ecx
  mov rdx, qword ptr {}[rip]
  mov r8, qword ptr {}[rip]
  xor r9d, r9d
  call [qword ptr __imp_MessageBoxW[rip]]
  test rax, rax
  jz .L_WIN_HANDLER
  mov qword ptr {}[rip], rax
  mov rcx, qword ptr {}[rip]
  call [qword ptr __imp_free[rip]]
  mov rcx, qword ptr {}[rip]
  call [qword ptr __imp_free[rip]]
  add rsp, 16
