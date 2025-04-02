	.file	"t.c"
	.intel_syntax noprefix
	.text
	.p2align 4
	.def	printf;	.scl	3;	.type	32;	.endef
	.seh_proc	printf
printf:
	push	rsi
	.seh_pushreg	rsi
	push	rbx
	.seh_pushreg	rbx
	sub	rsp, 56
	.seh_stackalloc	56
	.seh_endprologue
	lea	rsi, 88[rsp]
	mov	rbx, rcx
	mov	QWORD PTR 88[rsp], rdx
	mov	ecx, 1
	mov	QWORD PTR 96[rsp], r8
	mov	QWORD PTR 104[rsp], r9
	mov	QWORD PTR 40[rsp], rsi
	call	[QWORD PTR __imp___acrt_iob_func[rip]]
	mov	r8, rsi
	mov	rdx, rbx
	mov	rcx, rax
	call	__mingw_vfprintf
	add	rsp, 56
	pop	rbx
	pop	rsi
	ret
	.seh_endproc
	.section .rdata,"dr"
.LC0:
	.ascii "argv[0]: %s\12\0"
.LC1:
	.ascii "argv[1]: %s\12\0"
	.section	.text.startup,"x"
	.p2align 4
	.globl	main
	.def	main;	.scl	2;	.type	32;	.endef
	.seh_proc	main
main:
	push	rbx
	.seh_pushreg	rbx
	sub	rsp, 32
	.seh_stackalloc	32
	.seh_endprologue
	mov	rbx, rdx
	call	__main
	mov	rdx, QWORD PTR [rbx]
	lea	rcx, .LC0[rip]
	call	printf
	mov	rdx, QWORD PTR 8[rbx]
	lea	rcx, .LC1[rip]
	call	printf
	xor	eax, eax
	add	rsp, 32
	pop	rbx
	ret
	.seh_endproc
	.def	__main;	.scl	2;	.type	32;	.endef
	.ident	"GCC: (x86_64-posix-seh-rev1, Built by MinGW-Builds project) 14.2.0"
	.def	__mingw_vfprintf;	.scl	2;	.type	32;	.endef
