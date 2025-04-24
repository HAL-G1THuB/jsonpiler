.section .rdata, "dr"
.L__SEH_HANDLER_MSG:
  .ascii "An exception occurred!\nPossible causes:\n- Division by zero\n- stack overflow\n"
  .asciz "- null pointer dereference\n- invalid memory access\n- out-of-bounds array access\n..."
.data
