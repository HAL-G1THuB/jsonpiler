  lea rcx, qword ptr {}[rip]
  call .L_U8TO16
  mov qword ptr {}[rip], rax
  lea rcx, qword ptr {}[rip]
  call .L_U8TO16
  mov qword ptr {}[rip], rax
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
