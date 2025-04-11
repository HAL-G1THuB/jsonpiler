  xor ecx, ecx
  call [qword ptr __imp_ExitProcess[rip]]
  .seh_endproc
.L_WIN_HANDLER:
  call [qword ptr __imp_GetLastError[rip]]
  mov rbx, rax
  sub rsp, 32
  mov ecx, 0x1300
  xor edx, edx
  mov r8, rbx
  xor r9d, r9d
  lea rax, qword ptr .L_WIN_HANDLER_MSG[rip]
  mov qword ptr 0x20[rsp], rax
  mov qword ptr 0x28[rsp], 0
  mov qword ptr 0x30[rsp], 0
  call [qword ptr __imp_FormatMessageW[rip]]
  test rax, rax
  jz .L_EXIT
  xor ecx, ecx
  mov rdx, qword ptr .L_WIN_HANDLER_MSG[rip]
  xor r8d, r8d
  mov r9, 0x10
  call [qword ptr __imp_MessageBoxW[rip]]
.L_EXIT:
  mov rcx, qword ptr .L_WIN_HANDLER_MSG[rip]
  call [qword ptr __imp_LocalFree[rip]]
  mov rcx, rbx
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
