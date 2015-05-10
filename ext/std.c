#import <ctype.h>
#import <gc.h>
#import <stdio.h>
#import <stdlib.h>
#import <string.h>

#import "hummingbird.h"

/*
%TBuiltinConsole = type { void (i8*)* }
@Gconsole = global %TBuiltinConsole* null
*/

void TBuiltinConsole_mlog (void *s) {
  puts(s);
}

typedef struct BuiltinConsole {
  void (*log)(void *);
} TBuiltinConsole;

extern TBuiltinConsole *Gconsole;
TBuiltinConsole *Gconsole = &(TBuiltinConsole){
  .log = &TBuiltinConsole_mlog
};

// std.core.types.string.concat(string, string) -> string
char *Mstd_Mcore_Mtypes_Mstring_Fconcat(char *lvalue, char *rvalue) {
  size_t lvalueLen = strlen(lvalue);
  size_t rvalueLen = strlen(rvalue);
  size_t retLen    = lvalueLen + rvalueLen;
  void *ret = GC_malloc(retLen + 1);
  snprintf(ret, retLen + 1, "%s%s", lvalue, rvalue);
  return ret;
}

// std.core.types.string.uppercase(string) -> string
char *Mstd_Mcore_Mtypes_Mstring_Fuppercase(char *recv) {
  size_t recvLen = strlen(recv);
  char *ret = GC_malloc(recvLen + 1);
  for (unsigned int i = 0; i < recvLen; i++) {
    ret[i] = toupper(recv[i]);
  }
  ret[recvLen] = '\0';
  return ret;
}
char *Mstd_Mcore_Mtypes_Mstring_Flowercase(char *recv) {
  size_t recvLen = strlen(recv);
  char *ret = GC_malloc(recvLen + 1);
  for (unsigned int i = 0; i < recvLen; i++) {
    ret[i] = tolower(recv[i]);
  }
  ret[recvLen] = '\0';
  return ret;
}

// std.core.types.integer.toString(integer) -> string
hb_string Mstd_Mcore_Mtypes_Minteger_FtoString(hb_integer value) {
  char buffer[80];
  int len = snprintf(buffer, 80, "%lld", value);
  // Copy the buffer into a GC_malloc'ed one for returning
  void *ret = GC_malloc(len + 1);
  memcpy(ret, buffer, len + 1);
  return ret;
}

