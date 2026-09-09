; VINTAGE-1
; Author: roywalk3r
; Repo: https://github.com/roywalk3r/vintage
; License: MIT
; Dodge — the first interactive sprite game: steer the ship (sprite 0)
; with the arrows ($13 left / $14 right, 8px steps) while a rock (sprite 1)
; falls at random columns. Each fall that gets past you scores and speeds
; the game up; a fall that lands on your column crashes and resets the
; score. Score shows as two digit glyphs stamped into the fb plane (the
; sprite plane stays background-only, like every demo). The main loop
; polls $5802 for the frame tick — no IRQ, the rock ticks 2px per step.

FB     = $4000
KEY    = $5800
FRAME  = $5802
RND    = $5805
BEEPER = $5807

; --- zero page ---
PX     = $40      ; player x (y is fixed at 176)
PY     = $41      ; player y, never changes
RX     = $42      ; rock x
RY     = $43      ; rock y (2px per fall tick)
RSPEED = $44      ; frames per fall tick (floor 3)
RCNT   = $45      ; fall tick countdown
SCORE  = $46      ; dodges, 0..99 then rolls over
FRM    = $47      ; frame-tick compare
BLIP   = $48      ; beeper countdown frames
T0     = $49
T1     = $4A
T2     = $4B
DLO    = $4C      ; glyph source pointer
DHI    = $4D
ELO    = $4E      ; fb destination pointer
EHI    = $4F
TFLAG  = $50      ; scored-this-fall flag
T3     = $51
T4     = $52

 .org $E000

; --- init: sprites on, player centered, rock spawned, score drawn ---
start:
 lda #0
 sta BEEPER
 sta BLIP
 lda #<SHIP
 sta $580A
 lda #SHIP>>8
 sta $580B
 lda #<ROCK
 sta $580E
 lda #ROCK>>8
 sta $580F
 lda #3
 sta $5810        ; both sprites on

 lda #120
 sta PX
 lda #176
 sta PY
 lda RND          ; one read = one LFSR step
 and #$F8         ; 8px grid keeps the collision math exact
 sta RX
 lda #0
 sta RY
 sta TFLAG
 lda #4
 sta RSPEED
 sta RCNT
 lda #0
 sta SCORE
 sta FRM
 jsr drawscore
 lda PX
 sta $5808
 lda PY
 sta $5809
 lda RX
 sta $580C
 lda RY
 sta $580D

; --- main loop: wait for the frame tick, then update ---
main:
 lda FRAME
 cmp FRM
 beq main
 sta FRM
 jsr readkey
 jsr tickrock
 jsr tickblip
 lda PX
 sta $5808
 lda PY
 sta $5809
 lda RX
 sta $580C
 lda RY
 sta $580D
 jmp main

; --- keyboard: arrows step the player 8px, clamped to 0..248 ---
readkey:
 lda KEY
 beq rk_done
 cmp #$13
 bne rk_right
 lda PX
 sec
 sbc #8
 bcs rk_put
 lda #0
rk_right:
 cmp #$14
 bne rk_done
 lda PX
 clc
 adc #8
 bcs rk_max
 cmp #249
 bcc rk_put
rk_max:
 lda #248
rk_put:
 sta PX
rk_done:
 rts

; --- rock tick: falls 2px every RSPEED frames; the band 169..183 is the
; player's rows — crossing it scores, landing on the player's column in
; it crashes ---
tickrock:
 dec RCNT
 bne tr_done
 lda RSPEED
 sta RCNT
 lda RY
 clc
 adc #2
 sta RY
 cmp #169
 bcc tr_done      ; still above the band
 cmp #184
 bcc tr_band
 jmp respawn      ; fell past everything: new rock from the top
tr_band:
 ; crash if |PX-RX| < 8 (both sit on the 8px grid, so 8 = a clean miss)
 lda PX
 sec
 sbc RX
 bcs tr_abs
 eor #$FF
 clc
 adc #1
tr_abs:
 cmp #8
 bcs tr_dodge
 jmp crash
tr_dodge:
 lda TFLAG
 bne tr_done      ; already scored this fall
 lda #1
 sta TFLAG
 inc SCORE
 lda SCORE
 cmp #100
 bcc tr_sp
 lda #0           ; roll over at 99
 sta SCORE
tr_sp:
 lda RSPEED
 cmp #3
 bcc tr_bl        ; floor
 dec RSPEED
tr_bl:
 lda #60
 jsr blip60
 jsr drawscore
tr_done:
 rts

; --- crash: score resets, long low blip, fresh rock ---
crash:
 lda #0
 sta SCORE
 jsr drawscore
 lda #30
 jsr blip30
 jmp respawn

; --- respawn: random column, top of the screen ---
respawn:
 lda RND          ; one read = one LFSR step
 and #$F8
 sta RX
 lda #0
 sta RY
 sta TFLAG
 lda #1
 sta RCNT         ; fall again immediately
 rts

; --- beeper: dodge = short high blip, crash = long low blip ---
blip60:
 sta BEEPER
 lda #12
 sta BLIP
 rts

blip30:
 sta BEEPER
 lda #30
 sta BLIP
 rts

tickblip:
 lda BLIP
 beq tb_done
 dec BLIP
 bne tb_done
 lda #0
 sta BEEPER
tb_done:
 rts

; --- score as two digits stamped into the fb plane at (0,0) and (8,0);
; overwrite-in-place, no erase pass ---
drawscore:
 lda SCORE
 ldx #$FF
ds1:
 inx
 sec
 sbc #10
 bcs ds1
 adc #10          ; A = ones, X = tens (borrow undoes the last step)
 sta T3
 stx T4
 ldx #1
 lda T3
 jsr drawdigit
 ldx #0
 lda T4
 jsr drawdigit
 rts

; --- stamp one glyph: A = digit 0..9, X = fb column (0 or 1) ---
drawdigit:
 sta T0
 stx T1
 lda T0
 asl a
 asl a
 asl a            ; digit*8
 sta T0
 lda #<DGTS
 clc
 adc T0
 sta DLO
 lda #DGTS>>8
 adc #0
 sta DHI
 lda #<FB
 sta ELO
 lda #FB>>8
 sta EHI
 lda #0
 sta T2
dr_row:
 ldy T2
 lda (DLO),y      ; glyph row
 pha
 lda T2
 asl a
 asl a
 asl a
 asl a
 asl a            ; row*32
 clc
 adc T1           ; + column
 tay
 pla
 sta (ELO),y
 inc T2
 lda T2
 cmp #8
 bcc dr_row
 rts

; --- digits 0-9, rows bit-reversed (MSB-left), copied from the hello.s
; font8x8 so the score glyphs match every other demo ---
DGTS:
 .byte $7C,$C6,$CE,$DE,$F6,$E6,$7C,$00 ; '0'
 .byte $30,$70,$30,$30,$30,$30,$FC,$00 ; '1'
 .byte $78,$CC,$0C,$38,$60,$CC,$FC,$00 ; '2'
 .byte $78,$CC,$0C,$38,$0C,$CC,$78,$00 ; '3'
 .byte $1C,$3C,$6C,$CC,$FE,$0C,$1E,$00 ; '4'
 .byte $FC,$C0,$F8,$0C,$0C,$CC,$78,$00 ; '5'
 .byte $38,$60,$C0,$F8,$CC,$CC,$78,$00 ; '6'
 .byte $FC,$CC,$0C,$18,$30,$30,$30,$00 ; '7'
 .byte $78,$CC,$CC,$78,$CC,$CC,$78,$00 ; '8'
 .byte $78,$CC,$CC,$7C,$0C,$18,$70,$00 ; '9'

 .org $F000
SHIP:
 .byte $18,$18,$7E,$FF,$FF,$3C,$C3,$00
ROCK:
 .byte $3C,$7E,$FF,$DB,$FF,$66,$3C,$00

 .org $FFFC
 .word start