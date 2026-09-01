; VINTAGE-1
; Author: roywalk3r
; Repo: https://github.com/roywalk3r/vintage
; License: MIT
; Breakout — 4 rows x 8 columns of 32x8 px bricks tracked as four bitmask
; bytes in zero page (bit 7 = leftmost column), a 32-px paddle and a 2x2
; ball that steps every 2nd frame: 1 px vertically and HRATE px horizontally
; per step — quarter-paddle hits flatten the slope to 2, middle hits keep 45°.
; Rendering is incremental: bricks
; stamp at init and erase on break only, the paddle redraws only when it
; slides, the ball is eor-erased then eor-drawn through a (x & 7) mask.
; Every blip on $5807 decays via a zero-page counter after 8 frames.

FB     = $4000
KEY    = $5800
FRAMEL = $5802
RND    = $5805
BEEPER = $5807

; --- zero page (vars from $E0) ---
BRK = $E0    ; brick bits, 4 rows, bit 7 = leftmost column
PX  = $E4    ; paddle left byte column, 0..28
OPX = $E5    ; paddle column as last drawn
BX  = $E6    ; ball left x, 0..254
BY  = $E7    ; ball top y
DXF = $E8    ; 0 = left, 1 = right
DYF = $E9    ; 0 = up, 1 = down
SCD = $EA    ; ball step toggle: move on every 2nd frame
SFC = $EB    ; frames left until the beeper expires
TB  = $EC    ; ball byte column (BX >> 3)
TM  = $ED    ; ball eor mask for byte TB
TN  = $EE    ; ball eor mask for byte TB+1 (x&7 spill)
DLO = $EF    ; framebuffer pointer
DHI = $F0
T1  = $F1    ; scratch
T2  = $F2
TV  = $F3    ; fill value for the rect helpers
HRATE = $F4  ; horizontal px per ball step (1 = 45°, 2 = shallow fast)

 .org $E000

start: lda #$FF
 sta BRK
 sta BRK+1
 sta BRK+2
 sta BRK+3
 jsr clear_fb
 lda #$FF
 ldx #0
ib1: sta $4200,x
 sta $4300,x
 sta $4400,x
 sta $4500,x     ; 32 lit bricks fill the bands $4200..$45FF
 inx
 bne ib1
 lda #14
 sta PX
 sta OPX
 ldy PX
 lda #$FF
 jsr pad_fill
 jsr reset_ball
 jsr ball_sp     ; first draw pairs with the first erase in step_ball
 lda #0
 sta SCD
 sta SFC

main: jsr wait_frame
 lda SFC
 beq sf1
 dec SFC
 bne sf1
 lda #0
 sta BEEPER      ; blip decayed to silence
sf1: jsr keys
 lda PX
 cmp OPX
 beq nm1
 ldy OPX         ; paddle redraws only when it slides
 lda #0
 jsr pad_fill
 ldy PX
 lda #$FF
 jsr pad_fill
 lda PX
 sta OPX
nm1: inc SCD
 lda SCD
 cmp #2
 bcc main        ; ball steps on every 2nd frame
 lda #0
 sta SCD
 jsr step_ball
 jmp main

keys: lda KEY
 cmp #$13
 beq kdL
 cmp #$14
 beq kdR
 rts
kdL: lda PX
 beq kd0         ; clamp at column 0
 dec PX
 rts
kdR: lda PX
 cmp #28
 bcs kd0         ; clamp at column 28
 inc PX
kd0: rts

; --- erase ball, step, bounce walls/paddle/bricks, redraw ---
step_ball:
 jsr ball_sp     ; eor-erase the ball where it was
 lda DXF
 beq sbL
 clc
 lda BX
 adc HRATE       ; slide right by the slope rate
 bcs sbRw        ; wrapped past $FF: reflect
 cmp #255
 bcc sbRok       ; ≤ 254: still on screen
sbRw: eor #$FF   ; r=255→254, r=256→253: reflect about the wall plane
 sec
 sbc #2
 sta BX
 jmp sbW
sbRok: sta BX
 jmp sbV
sbL: sec
 lda BX
 sbc HRATE
 bcs sbLok       ; ≥ 0: ball stays on screen
 eor #$FF        ; < 0: reflect
 clc
 adc #1          ; r=-1→1, r=-2→2
 sta BX
 jmp sbW
sbLok: sta BX
 jmp sbV
sbW: lda DXF
 eor #1
 sta DXF
 lda #91
 jsr sfx
sbV: lda DYF
 bne sbFall      ; rise first, fall below: keeps every branch within range
 dec BY          ; rise
 lda BY
 bne sbUp        ; not at the ceiling: on to the bricks
 lda #1          ; ceiling: reverse without stepping
 sta DYF
 lda #91
 jsr sfx
sbUp: jmp sbBr
sbFall:
 inc BY          ; fall
 lda BY
 cmp #191
 bcs sbLost      ; fell past line 192
 cmp #177
 bcc sbBr        ; above the paddle zone, keep falling
 lda PX          ; paddle zone: overlap test in pixels
 asl a
 asl a
 asl a
 sta T1          ; paddle left edge
 lda BX
 clc
 adc #1
 sec
 sbc T1
 bcc sbBr        ; ball right edge left of the paddle
 lda T1
 clc
 adc #31
 sta T2
 lda BX
 sec
 sbc T2
 bcs sbBr        ; ball left edge right of the paddle
 lda #0
 sta DYF
 lda #176
 sta BY          ; park the ball on top of the paddle
 lda #1
 sta HRATE       ; default 45°, quarter hits flatten to 2
 lda BX
 sec
 sbc T1          ; o = ball left edge - paddle face (0..32)
 bpl sbP1
 lda #0          ; a 1px graze lands in the far-left tier
sbP1: ldx #1        ; aim right by default (right half of the face)
 ldy #1
 cmp #16
 bcs sbP3        ; o ≥ 16: right half
 ldx #0          ; left half → aim left
 cmp #8
 bcs sbP4        ; 8..15: 45° left
 ldy #2          ; 0..7: shallow left
sbP4: stx DXF
 sty HRATE
 jmp sbPS
sbP3: cmp #24
 bcc sbP4        ; 16..23: 45° right
 ldy #2          ; 24..32: shallow right
 jmp sbP4
sbPS: lda #136
 jsr sfx
 jmp sbBr
sbLost:
 lda #255
 jsr sfx         ; 235 Hz blip, then respawn above the paddle
 jsr reset_ball
sbBr: lda BY
 cmp #16         ; brick rows span scanlines 16..47
 bcc sbDone
 cmp #48
 bcs sbDone
 sec
 sbc #16
 lsr a
 lsr a
 lsr a
 tax             ; brick row
 lda BX
 lsr a
 lsr a
 lsr a
 lsr a
 lsr a
 tay
 lda BMASK,y
 sta T2
 lda BRK,x
 and T2
 beq sbDone      ; brick already broken
 eor T2
 sta BRK,x
 lda #0
 sta TV
 lda BX
 lsr a
 lsr a
 lsr a
 lsr a
 lsr a
 jsr brick_sp    ; A = brick column, X = row
 lda DYF
 eor #1
 sta DYF
 lda #68
 jsr sfx
sbDone: jsr ball_sp
 rts

; --- respawn centred above the paddle, random horizontal direction ---
reset_ball:
 lda #126
 sta BX
 lda #170
 sta BY
 lda #1
 sta DYF
 lda RND
 and #1
 sta DXF
 lda #1
 sta HRATE       ; respawn at 45°
 rts

; --- raise a blip: half-period in A, 8-frame decay handled in main ---
sfx:
 sta BEEPER
 lda #8
 sta SFC
 rts

; --- eor the two ball pixels into one scanline (scanline in A) ---
brow:
 sta T1
 lsr a
 lsr a
 lsr a
 clc
 adc #$40
 sta DHI
 lda T1
 asl a
 asl a
 asl a
 asl a
 asl a
 clc
 adc TB
 sta DLO
 ldy #0           ; DLO already holds the byte column: index 0
 lda (DLO),y
 eor TM
 sta (DLO),y
 lda TN
 beq bw0
 ldy #1
 lda (DLO),y
 eor TN
 sta (DLO),y
bw0: rts

; --- toggle the 2x2 ball at (BX,BY) ---
ball_sp:
 lda BX
 lsr a
 lsr a
 lsr a
 sta TB
 lda BX
 and #7
 tax
 lda MTAB,x
 sta TM
 lda NTAB,x
 sta TN
 lda BY
 jsr brow
 lda BY
 clc
 adc #1
 jsr brow
 rts

; --- X = brick row, A = column, fill value in TV; one 32x8 px block ---
brick_sp:
 asl a
 asl a
 sta DLO
 txa
 clc
 adc #$42
 sta DHI
 ldy #0
 lda TV
 ldx #8
bs1: sta (DLO),y
 iny
 sta (DLO),y
 iny
 sta (DLO),y
 iny
 sta (DLO),y
 iny
 lda DLO
 clc
 adc #32
 sta DLO         ; next scanline
 tya
 sec
 sbc #4
 tay
 dex
 bne bs1
 rts

; --- A = value, Y = left byte column; 12 stores redraw the 32-px paddle ---
pad_fill:
 sta $5640,y
 sta $5641,y
 sta $5642,y
 sta $5643,y
 sta $5660,y
 sta $5661,y
 sta $5662,y
 sta $5663,y
 sta $5680,y
 sta $5681,y
 sta $5682,y
 sta $5683,y
 rts

; --- blank all 24 framebuffer pages (init only) ---
clear_fb:
 lda #$40
 sta DHI
 lda #0
 sta DLO
 tay             ; Y = 0, A stays 0 for the inner loop
cf1: sta (DLO),y
 iny
 bne cf1
 inc DHI
 ldx DHI
 cpx #$58
 bne cf1
 rts

; --- wait for the low frame-counter byte to change (tune.s style) ---
wait_frame:
 lda FRAMEL
wf1: cmp FRAMEL
 beq wf1
 rts

; --- ball masks: two bits at x&7, the second spills into byte TB+1 at x&7 = 7 ---
MTAB: .byte $C0,$60,$30,$18,$0C,$06,$03,$01
NTAB: .byte 0,0,0,0,0,0,0,$80
BMASK: .byte $80,$40,$20,$10,$08,$04,$02,$01

 .org $FFFC
 .word start
 .org $FFFE
 .word start