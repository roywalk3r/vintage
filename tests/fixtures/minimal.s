; CLI smoke fixture: one lit byte in the framebuffer, then spin.
 .org $E000
start:
 lda #$81
 sta $4000
hold: jmp hold
 .org $FFFC
 .word start