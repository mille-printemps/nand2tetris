// Push a segment

// For "local", "argument", "this" and "that"
pub const SEGMENT: &str = r#"@{segment}
D=M
@{index}
A=D+A
D=M"#;

// For "temp"
pub const TEMP: &str = r#"@5
D=A
@{index}
A=D+A
D=M"#;

// For "pointer"
pub const POINTER: &str = r#"@{segment}
A=M
D=A"#;

// For "static"
pub const STATIC: &str = r#"@{file}.{index}
A=M
D=A"#;

// For "constant"
pub const CONSTANT: &str = r#"@{index}
D=A"#;

// Push to the stack
pub const POST_PUSH: &str = r#"@SP
A=M
M=D
@SP
M=M+1"#;

// Pop a segment

// Calculate adress of "segment" "index"
pub const SEGMENT_ADDRESS: &str = r#"@{segment}
D=M
@{index}
D=D+A"#;

// Calculate address of temp "index"
pub const TEMP_ADDRESS: &str = r#"@5
D=A
@{index}
D=D+A"#;

// Calculate address of pointer "index"
pub const POINTER_ADDRESS: &str = r#"@{segment}
D=A"#;

// Calculate address of static "index"
pub const STAIC_ADDRESS: &str = r#"@{file}.{index}
D=A"#;

// Calculate address of "index" of "segment"
pub const PRE_POP: &str = r#"@SP
M=M-1
A=M
D=M
@SP
M=M+1
A=M
M=D
@SP
M=M-1"#;

// Pop from the stack
pub const POST_POP: &str = r#"@SP
A=M
M=D
@SP
M=M+1
@SP
A=M
D=M
@SP
M=M-1
A=M
A=M
M=D"#;

// Arithmetic and logic instructions

// For "add", "sub", "and" and "or"
pub const BINARY_COMP: &str = r#"@SP
M=M-1
A=M
D=M
@SP
M=M-1
A=M
{comp}
@SP
A=M
M=D
@SP
M=M+1"#;

// For "not", "neg"
pub const UNARY_COMP: &str = r#"@SP
M=M-1
A=M
D=M
{comp}
@SP
A=M
M=D
@SP
M=M+1"#;

// For "eq", "lt" and "gt"
pub const COMPARISON: &str = r#"@SP
M=M-1
A=M
D=M
@SP
M=M-1
A=M
D=D-M
@SP
A=M
M=-1
@{label}
D;{jump}
@SP
A=M
M=!M
({label})
@SP
M=M+1"#;

// For "if-goto"
pub const IF_GOTO: &str = r#"@SP
M=M-1
A=M
D=M
@{dontgoto}
D;JEQ
@{label}
0;JMP
({dontgoto})"#;

// For "function"
pub const FUNCTION: &str = r#"@0
D=A
@SP
A=M
M=D
@SP
M=M+1"#;

// For "call"
pub const CALL: &str = r#"// save return address to stack
@{caller}$ret.{callee_index}
D=A
@SP
A=M
M=D
@SP
M=M+1
// save LCL
@LCL
D=M
@SP
A=M
M=D
@SP
M=M+1
// save ARG
@ARG
D=M
@SP
A=M
M=D
@SP
M=M+1
// save THIS
@THIS
D=M
@SP
A=M
M=D
@SP
M=M+1
// save THAT
@THAT
D=M
@SP
A=M
M=D
@SP
M=M+1
// ARG = SP - 5 - nArgs
D=M
@5
D=D-A
@{nargs}
D=D-A
@ARG
M=D
// set LCL to SP
@SP
D=M
@LCL
M=D
// jump to {callee}
@{callee}
0;JMP
({caller}$ret.{callee_index})"#;

// For "return"
pub const RETURN: &str = r#"// endFrame (R13) = LCL
@LCL
D=M
@R13
M=D
// retAddr (R14) = *(endFrame - 5)
@5
A=D-A
D=M
@R14
M=D
// *ARG = pop()
@SP
M=M-1
A=M
D=M
@ARG
A=M
M=D
// SP = ARG + 1
@ARG
D=M
@SP
M=D+1
// THAT = *(endFrame - 1)
@R13
M=M-1
A=M
D=M
@THAT
M=D
// THIS = *(endFrame - 2)
@R13
M=M-1
A=M
D=M
@THIS
M=D
// ARG = *(endFrame - 3)
@R13
M=M-1
A=M
D=M
@ARG
M=D
// LCL = *(endFrame - 4)
@R13
M=M-1
A=M
D=M
@LCL
M=D
// goto retAddr
@R14
A=M
0;JMP"#;

pub const BOOTSTRAP: &str = r#"@256
D=A
@SP
M=D"#;
