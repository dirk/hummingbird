#import <ctype.h>
#import <stdlib.h>
#import <stdio.h>
#import <string.h>
#import <gc.h>

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

