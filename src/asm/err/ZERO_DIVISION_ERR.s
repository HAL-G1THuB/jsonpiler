.L__ZERO_DIVISION_ERR:
  xor ecx, ecx
  lea rdx, qword ptr .L__ZERO_DIVISION_MSG[rip]
  xor r8d, r8d
  mov r9d, 0x10
  call [qword ptr __imp_MessageBoxA[rip]]
  mov ecx, -1
  call [qword ptr __imp_ExitProcess[rip]]
