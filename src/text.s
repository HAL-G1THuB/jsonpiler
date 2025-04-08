  xor ecx, ecx
  call ExitProcess
  .seh_endproc
display_error:
  call GetLastError
  mov rbx, rax
  sub rsp, 32
  mov ecx, 0x1300
  xor edx, edx
  mov r8, rbx
  xor r9d, r9d
  lea rax, QWORD PTR EMSG[rip]
  mov QWORD PTR 0x20[rsp], rax
  mov QWORD PTR 0x28[rsp], 0
  mov QWORD PTR 0x30[rsp], 0
  call FormatMessageW
  test rax, rax
  jz exit_program
  xor ecx, ecx
  mov rdx, QWORD PTR EMSG[rip]
  xor r8d, r8d
  mov r9, 0x10
  call MessageBoxW
exit_program:
  mov rcx, QWORD PTR EMSG[rip]
  call LocalFree
  mov rcx, rbx
  call ExitProcess
exception_handler:
  sub rsp, 40
  xor ecx, ecx
  lea rdx, msg_text[rip]
  xor r8d, r8d
  mov r9d, 16
  call MessageBoxA
  mov ecx, -1
  call ExitProcess
