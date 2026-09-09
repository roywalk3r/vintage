; VINTAGE-1
; Author: roywalk3r
; Repo: https://github.com/roywalk3r/vintage
; License: MIT
; Sprites — the sprite unit and the vsync IRQ, exercised together: two
; 8x8 sprites bounce off the video edges, with the whole motion loop
; running out of the IRQ handler (the main loop is a bare jmp). Bounces
; blip the beeper (X walls low, Y walls high). The background is a 4px
; checkerboard, so the XOR compositing is visible wherever a sprite
; crosses lit cells. Velocities are signed 8-bit and capped at 7: the
; bounce math relies on X+V never wrapping past 255 at the right wall.

FB     = $4000
KEY    = $5800
FRAME  = $5802
BEEPER = $5807
SX     = $40      ; 2 bytes: sprite 0/1 x latches (pixels)
SY     = $42      ; 2 bytes: y latches
VX     = $44      ; 2 bytes: signed x velocities
VY     = $46      ; 2 bytes: signed y velocities
BLIP   = $48      ; frames left before the beeper silences
ROWPAR = $49      ; checkerboard row parity (scratch)
PAR    = $4A      ; cell parity (scratch)
DLO    = $4B      ; checkerboard row pointer
DHI    = $4C
ROWNUM = $4D      ; checkerboard row counter (scratch)

 .org $E000

; --- init: checkerboard, sprite latches + patterns, then cli ---
start:
 lda #0
 sta BEEPER
 jsr checker

 lda #<BALL
 sta $580A
 lda #BALL>>8
 sta $580B
 lda #<RING
 sta $580E
 lda #RING>>8
 sta $580F
 lda #3
 sta $5810        ; both sprites on

 lda #16
 sta SX
 lda #200
 sta SX+1
 lda #24
 sta SY
 lda #150
 sta SY+1
 lda #2
 sta VX
 lda #3
 sta VY
 lda #$FD         ; -3
 sta VX+1
 lda #$FE         ; -2
 sta VY+1

 cli

; --- idle: the IRQ handler owns every frame ---
main:
 jmp main

; --- vsync IRQ: move both sprites, run the blip countdown ---
irq:
 pha
 txa
 pha
 ldx #0
 jsr bounce
 jsr bouncey
 ldx #1
 jsr bounce
 jsr bouncey
 ; publish the new positions: composition reads the latches, not ZP
 lda SX
 sta $5808
 lda SY
 sta $5809
 lda SX+1
 sta $580C
 lda SY+1
 sta $580D
 lda BLIP
 beq irq1
 dec BLIP
 bne irq1
 lda #0
 sta BEEPER
irq1:
 pla
 tax
 pla
 rti

; --- bounce one sprite (X = 0 or 1) off all four walls ---
; Moving right, X+VX can reach 248+7 = 255 (no wrap: carry clear, the
; cmp catches it). Moving left, a clear carry from the adc is the
; borrow — the only way a leftward step can land in $80-$FF territory.
bounce:
 lda VX,x
 bmi bxl
 lda SX,x
 clc
 adc VX,x
 bcs brc          ; wrapped past 255: clamp
 cmp #249
 bcc bxs
brc:
 lda #248
 sta SX,x
 jsr flipvx
 lda #100         ; low blip
 jsr blip
 rts
bxs:
 sta SX,x
 rts
bxl:
 lda SX,x
 clc
 adc VX,x
 bcc blc          ; borrow: wrapped below 0
 sta SX,x
 rts
blc:
 lda #0
 sta SX,x
 jsr flipvx
 lda #100
 jsr blip
 rts

; --- vertical bounce: walls at 0 and 184 (rows 184..191 are the
; sprite's last full row) ---
bouncey:
 lda VY,x
 bmi byl
 lda SY,x
 clc
 adc VY,x
 bcs byc          ; wrapped past 255: clamp
 cmp #185
 bcc bys
byc:
 lda #184
 sta SY,x
 jsr flipvy
 lda #50          ; high blip
 jsr blip
 rts
bys:
 sta SY,x
 rts
byl:
 lda SY,x
 clc
 adc VY,x
 bcc bylc         ; borrow: wrapped below 0
 sta SY,x
 rts
bylc:
 lda #0
 sta SY,x
 jsr flipvy
 lda #50
 jsr blip
 rts

; --- negate VX,x (two's complement) ---
flipvx:
 lda VX,x
 eor #$FF
 clc
 adc #1
 sta VX,x
 rts

flipvy:
 lda VY,x
 eor #$FF
 clc
 adc #1
 sta VY,x
 rts

; --- beeper on for 12 frames at the period in A ---
blip:
 sta BEEPER
 lda #12
 sta BLIP
 rts

; --- fill the fb with a 4px checkerboard: byte value alternates
; $F0/$0F on the parity of (col>>1) ^ (row>>2) ---
checker:
 lda #<FB
 sta DLO
 lda #FB>>8
 sta DHI
 lda #0
 sta ROWNUM
ck_row:
 lda ROWNUM
 lsr a
 lsr a
 and #1
 sta ROWPAR
 ldy #0
ck_b:
 tya
 lsr a
 and #1
 eor ROWPAR
 beq ck_even
 lda #$0F
 bne ck_put
ck_even:
 lda #$F0
ck_put:
 sta (DLO),y
 iny
 cpy #32
 bne ck_b
 ; advance the row pointer 32 bytes; 192 rows span 1.5 pages
 lda DLO
 clc
 adc #32
 sta DLO
 bcc ck_nx
 inc DHI
ck_nx:
 inc ROWNUM
 lda ROWNUM
 cmp #192
 bcc ck_row
 rts

; --- sprite patterns (8 bytes, MSB leftmost, bus-fetched at vsync) ---
 .org $F000
BALL:
 .byte $3C,$7E,$FF,$FF,$FF,$FF,$7E,$3C
RING:
 .byte $81,$C3,$66,$3C,$3C,$66,$C3,$81

 .org $FFFA
 .word irq, start, irq