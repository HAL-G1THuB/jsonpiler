	lea	rcx,	{msg}
	call	.L__U8TO16
	mov	rdi,	rax
	lea	rcx,	{title}
	call	.L__U8TO16
	mov	rsi,	rax
	xor	ecx,	ecx
	mov	rdx,	rdi
	mov	r8,	rsi
	xor	r9d,	r9d
	call	[qword	ptr	__imp_MessageBoxW[rip]]
	test	rax,	rax
	jz	.L__WIN_HANDLER
	mov	{ret},	rax
	mov	rcx,	rdi
	call	[qword	ptr	__imp_free[rip]]
	mov	rcx,	rsi
	call	[qword	ptr	__imp_free[rip]]
