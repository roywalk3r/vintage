; VINTAGE-1
; Author: roywalk3r
; Repo: https://github.com/roywalk3r/vintage
; License: MIT
; Two-bank fixture: boot in bank 0, work in bank 1, tied together by the
; $E005 trampoline (bank 1 is visible from the very next fetch after
; sta $5806).

        .org $E000
boot:   lda #1
        sta $5806        ; bank 1 is visible from the very next fetch

 .bank 1
        .org $E005       ; trampoline: execution continues here, in bank 1
        jmp $F000

        .org $F000
        lda #$81
        sta $4000        ; row 0, x=0..7 lit
hang:   jmp hang

 .bank 0
        .org $FFFA
        .word boot, boot, boot