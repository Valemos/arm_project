target extended-remote :3333

set confirm off

# print demangled symbols
set print asm-demangle on

# set backtrace limit to not have infinite backtrace loops
set backtrace limit 32

# detect unhandled exceptions, hard faults and panics
break DefaultHandler
break HardFault
break rust_begin_unwind

monitor tpiu config internal itm.txt uart off 8000000
monitor itm port 0 on

# *try* to stop at the user entry point (it might be gone due to inlining)
break main

load
continue
step
