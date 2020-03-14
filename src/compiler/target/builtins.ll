declare i32 @printf(i8* nocapture, ...) nounwind

@builtin_println_int64_fmt = private unnamed_addr constant [4 x i8] c"%d\0A\00"

define external i64 @builtin_println_int64(i64 %value) {
    %deref = getelementptr [4 x i8], [4 x i8]* @builtin_println_int64_fmt, i32 0, i32 0
    call i32 (i8*, ...) @printf(i8* %deref, i64 %value)
    ret i64 0
}
