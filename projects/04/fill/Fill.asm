// This file is part of www.nand2tetris.org
// and the book "The Elements of Computing Systems"
// by Nisan and Schocken, MIT Press.
// File name: projects/04/Fill.asm

// Runs an infinite loop that listens to the keyboard input.
// When a key is pressed (any key), the program blackens the screen,
// i.e. writes "black" in every pixel;
// the screen should remain fully black as long as the key is pressed.
// When no key is pressed, the program clears the screen, i.e. writes
// "white" in every pixel;
// the screen should remain fully clear as long as no key is pressed.

// Put your code here.

(LOOP)
    // Map the screen start location to RAM0
    @SCREEN
    D=A
    @0
    M=D

(KBDPRESSED)
    @KBD
    D=M
    // If a key is pressed, goto BLACK
    @BLACK
    D;JGT
    // Otherwise, goto WHITE
    @WHITE
    D;JEQ
    // Check if a key is pressed
    @KBDPRESSED
    0;JMP

(BLACK)
    // Save data to fill for black
    @1
    M=-1
    @FILL
    0;JMP

(WHITE)
    // Save data to fill for white
    @1
    M=0
    @FILL
    0;JMP

(FILL)
    // Check which should be used, 0 or 1
    @1
    D=M
    // Save the current address of the pixel, then fill it
    @0
    A=M
    M=D
    // Calculate the rest of pixels to be filled by subtracting
    // the next address of the pixel from RAM[KBD]
    @0
    D=M+1
    @KBD
    D=A-D
    // Update the current address of the pixel to be filled
    @0
    M=M+1
    // If the rest of pixels remains, goto FILL
    @FILL
    D;JGT

    @LOOP
    0;JMP
