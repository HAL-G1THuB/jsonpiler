.section .rdata, "dr"
.L__SEH_HANDLER_MSG:
  .ascii "An exception occurred!\nPossible causes:\n"
  .ascii "- Division by zero\n- invalid memory access\n"
  .ascii "- null pointer dereference\n- stack overflow\n"
  .asciz "- invalid memory access\n- out-of-bounds array access\n..."
.data
