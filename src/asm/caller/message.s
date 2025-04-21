  lea rcx, qword ptr {msg}[rip]
  call .L__U8TO16
  mov rdi, rax
  lea rcx, qword ptr {title}[rip]
  call .L__U8TO16
  mov rsi, rax
  xor ecx, ecx
  mov rdx, rdi
  mov r8, rsi
  xor r9d, r9d
  call [qword ptr __imp_MessageBoxW[rip]]
  test rax, rax
  jz .L__WIN_HANDLER
  mov qword ptr {ret}[rip], rax
  mov rcx, rdi
  call [qword ptr __imp_free[rip]]
  mov rcx, rsi
  call [qword ptr __imp_free[rip]]
