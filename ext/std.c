#import <stdlib.h>
#import <stdio.h>

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

