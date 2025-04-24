  mov rbp, rsp
  .seh_setframe rbp, 0
  sub rsp, {size}
  .seh_stackalloc {size}
  .seh_endprologue
  .seh_handler .L__SEH_HANDLER, @except