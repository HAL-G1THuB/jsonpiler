	mov	ecx,	65001
	call	[qword	ptr	__imp_SetConsoleCP[rip]]
	test	eax,	eax
	jz	.L__WIN_HANDLER
	mov	ecx,	65001
	call	[qword	ptr	__imp_SetConsoleOutputCP[rip]]
	test	eax,	eax
	jz	.L__WIN_HANDLER
	mov	ecx,	-10
	call	[qword	ptr	__imp_GetStdHandle[rip]]
	cmp	rax,	-1
	je	.L__WIN_HANDLER
	mov	qword	ptr	.L__STDI[rip],	rax
	mov	ecx,	-11
	call	[qword	ptr	__imp_GetStdHandle[rip]]
	cmp	rax,	-1
	je	.L__WIN_HANDLER
	mov	qword	ptr	.L__STDO[rip],	rax
	mov	ecx,	-12
	call	[qword	ptr	__imp_GetStdHandle[rip]]
	cmp	rax,	-1
	je	.L__WIN_HANDLER
	mov	qword	ptr	.L__STDE[rip],	rax
	call	[qword	ptr	__imp_GetProcessHeap[rip]]
	test	rax,	rax
	jz	.L__WIN_HANDLER
	mov	qword	ptr	.L__HEAP[rip],	rax
