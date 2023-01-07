// This file is part of www.nand2tetris.org
// and the book "The Elements of Computing Systems"
// by Nisan and Schocken, MIT Press.
// File name: projects/04/Mult.asm

// Multiplies R0 and R1 and stores the result in R2.
// (R0, R1, R2 refer to RAM[0], RAM[1], and RAM[2], respectively.)
//
// This program only needs to handle arguments that satisfy
// R0 >= 0, R1 >= 0, and R0*R1 < 32768.

// Put your code here.

    // sum = 0
    @sum
    M=0
    // If R0 <= 0, goto STOP
    @R0
    D=M
    @STOP
    D;JLE
    // If R1 <= 0, goto STOP
    @R1
    D=M
    @STOP
    D;JLE

(LOOP)
    // sum = sum + R0
    @sum
    D=M
    @R0
    D=D+M
    @sum
    M=D
    // R1 = R1 - 1
    @R1
    D=M
    @1
    D=D-A
    M=D
    // If R1 == 0, goto STOP
    @R1
    D=M
    @STOP
    D;JEQ
    // If 32767 == sum, goto STOP
    @sum
    D=M
    @32767
    D=D-A
    @STOP
    D;JEQ

    @LOOP
    0;JMP

(STOP)
    // R2 = sum
    @sum
    D=M
    @R2
    M=D

(END)
    @END
    0;JMP