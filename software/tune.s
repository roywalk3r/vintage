; VINTAGE-1
; Author: roywalk3r
; Repo: https://github.com/roywalk3r/vintage
; License: MIT
; Beeper tune — plays a 32-note melody through $5807. Each note lasts 12
; frames (~200 ms); $5807 holds the square-wave half-period in CPU cycles
; (0 = silence). Runs forever.

 .org $E000

NOTE  = $E0
PHASE = $E1

BEEPER = $5807
FRAMEL = $5802

start:
 lda #0
 sta NOTE          ; note index
 sta PHASE         ; frames elapsed in current note

; --- one frame of the tune ---------------------------------------------
loop:
 jsr wait_frame
 inc PHASE
 lda PHASE
 cmp #12           ; 12 frames = ~200 ms per note
 bcc loop          ; keep sounding this note

 lda #0            ; advance to the next note
 sta PHASE
 inc NOTE
 lda NOTE
 cmp #32
 bcc play
 lda #0            ; loop the melody
 sta NOTE

play:
 lda NOTE          ; NOTE*2 -> index into the period table
 asl a
 tax
 lda tabl,X        ; 0 entries are rests: writing 0 silences
 sta BEEPER
 jmp loop

; --- wait for the frame counter to change -------------------------------
wait_frame:
 lda FRAMEL
wait1:
 cmp FRAMEL
 beq wait1
 rts

; --- half-period table (32 notes / rests) -------------------------------
; freq = 120000 / (2 * n)  ->  n = 60000 / freq
tabl:
 .byte 0           ; rest
 .byte 227         ; C4
 .byte 202         ; D4
 .byte 180         ; E4
 .byte 170         ; F4
 .byte 152         ; G4
 .byte 135         ; A4
 .byte 120         ; B4
 .byte 113         ; C5
 .byte 0           ; rest
 .byte 227         ; C4
 .byte 202         ; D4
 .byte 180         ; E4
 .byte 170         ; F4
 .byte 152         ; G4
 .byte 135         ; A4
 .byte 120         ; B4
 .byte 113         ; C5
 .byte 0           ; rest
 .byte 180         ; E4
 .byte 180         ; E4
 .byte 0           ; rest
 .byte 180         ; E4
 .byte 0           ; rest
 .byte 227         ; C4
 .byte 180         ; E4
 .byte 152         ; G4
 .byte 0           ; rests pad out the 32
 .byte 0
 .byte 0
 .byte 0
 .byte 0

 .org $FFFC
 .word start
 .org $FFFE
 .word start