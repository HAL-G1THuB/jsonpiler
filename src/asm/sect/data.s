.section	.rdata, "dr"
  .L__SEH_HANDLER_MSG:
  .ascii "An exception occurred!\nPossible causes:\n- Division by zero\n"
  .ascii "- null pointer dereference\n- invalid memory access\n- out-of-bounds array access\n"
  .asciz "- invalid input or arguments\n- stack overflow\n- file not found\n..."
.data
