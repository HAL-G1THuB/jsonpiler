.section	.rdata, "dr"
  .L_SEH_HANDLER_MSG:
  .ascii "An exception occurred!\nPossible causes:\n"
  .ascii "- Division by zero\n- null pointer dereference\n"
  .ascii "- invalid memory access\n- out-of-bounds array access\n"
  .ascii "- invalid input or arguments\n- stack overflow\n"
  .asciz "- file not found\n..."
.data
