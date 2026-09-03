// Wrapper header for bindgen
// Undefine stdout macro to avoid conflict with field name in libriscv.h
#ifdef stdout
#undef stdout
#endif

#include "libriscv-c/c/libriscv.h"
