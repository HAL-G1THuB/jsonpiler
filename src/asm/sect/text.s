.L__WIN_HANDLER:
  call [qword ptr __imp_GetLastError[rip]]
  mov ebx, eax
  sub rsp, 32
  mov ecx, 0x1300
  xor edx, edx
  mov r8d, ebx
  xor r9d, r9d
  lea rax, qword ptr .L__WIN_HANDLER_MSG[rip]
  mov qword ptr 0x20[rsp], rax
  mov dword ptr 0x28[rsp], 0
  mov dword ptr 0x30[rsp], 0
  call [qword ptr __imp_FormatMessageW[rip]]
  test eax, eax
  jz .L__EXIT
  xor ecx, ecx
  mov rdx, qword ptr .L__WIN_HANDLER_MSG[rip]
  xor r8d, r8d
  mov r9d, 0x10
  call [qword ptr __imp_MessageBoxW[rip]]
.L__EXIT:
  mov rcx, qword ptr .L__WIN_HANDLER_MSG[rip]
  call [qword ptr __imp_LocalFree[rip]]
  mov ecx, ebx
  call [qword ptr __imp_ExitProcess[rip]]
.L__SEH_HANDLER:
  sub rsp, 40
  xor ecx, ecx
  lea rdx, .L__SEH_HANDLER_MSG[rip]
  xor r8d, r8d
  mov r9d, 0x10
  call [qword ptr __imp_MessageBoxA[rip]]
  mov ecx, -1
  call [qword ptr __imp_ExitProcess[rip]]
