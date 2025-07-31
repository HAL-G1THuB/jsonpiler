.L__WIN_HANDLER:
	call	[qword	ptr	__imp_GetLastError[rip]]
	mov	edi,	eax
	mov	ecx,	0x1300
	xor	edx,	edx
	mov	r8d,	edi
	xor	r9d,	r9d
	lea	rax,	qword	ptr	0x38[rsp]
	mov	qword	ptr	0x20[rsp],	rax
	mov	dword	ptr	0x28[rsp],	0
	mov	dword	ptr	0x30[rsp],	0
	call	[qword	ptr	__imp_FormatMessageW[rip]]
	test	eax,	eax
	jz	.L__EXIT
	xor	ecx,	ecx
	mov	rdx,	qword	ptr	0x38[rsp]
	xor	r8d,	r8d
	mov	r9d,	0x10
	call	[qword	ptr	__imp_MessageBoxW[rip]]
.L__EXIT:
	mov	rcx,	qword	ptr	0x38[rsp]
	call	[qword	ptr	__imp_LocalFree[rip]]
	mov	ecx,	edi
	call	[qword	ptr	__imp_ExitProcess[rip]]
.L__SEH_HANDLER:
	xor	ecx,	ecx
	lea	rdx,	qword	ptr	.L__SEH_HANDLER_MSG[rip]
	xor	r8d,	r8d
	mov	r9d,	0x10
	call	[qword	ptr	__imp_MessageBoxA[rip]]
	mov	ecx,	-1
	call	[qword	ptr	__imp_ExitProcess[rip]]
